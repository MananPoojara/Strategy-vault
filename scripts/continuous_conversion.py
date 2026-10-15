#!/usr/bin/env python3
"""Refill bounded book-conversion/review jobs from an owner-authorized private backlog.

This coordinator never publishes. Independent results remain pending integration.
Original source/data and the authoritative checkout are not writable by agents.
"""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

import overnight_worker as worker
from publish_reviewed_batch import allowed_file


class UsageLimit(Exception):
    pass


class Stopped(Exception):
    pass


def available_disk_bytes(path):
    return shutil.disk_usage(path).free


def sha256(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for chunk in iter(lambda:f.read(1024*1024),b''):h.update(chunk)
    return h.hexdigest()


def load_backlog(path):
    value=json.loads(Path(path).read_text())
    if value.get('schema_version')!=1 or not value.get('approved_by'):
        raise ValueError('Backlog needs owner authorization and schema_version 1')
    tasks=value.get('tasks');seen=set()
    if not isinstance(tasks,list) or len(tasks)>100000:
        raise ValueError('Backlog must contain a bounded task list')
    for task in tasks:
        key=task.get('id','')
        if not re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9_.-]{0,63}',key) or key in seen:
            raise ValueError('Backlog needs unique safe task IDs')
        seen.add(key)
        for field in ('author','book','scope','instrument'):
            if not isinstance(task.get(field),str) or not task[field].strip():
                raise ValueError('Backlog missing '+field)
        for field in ('source_paths','data_paths','adaptations','method_ids'):
            if not isinstance(task.get(field),list):raise ValueError('Backlog missing '+field)
        if not task['source_paths'] or not isinstance(task.get('source_hashes'),dict):
            raise ValueError('Actual source paths and hashes are required')
        for name in task['source_paths']+task['data_paths']:
            if not isinstance(name,str) or not Path(name).is_absolute():
                raise ValueError('Input paths must be absolute')
    return value


def verify_inputs(task):
    for name in task['source_paths']:
        path=Path(name);expected=task['source_hashes'].get(name)
        if not path.is_file() or not expected or sha256(path)!=expected:
            raise ValueError('Actual source hash changed or source unavailable: '+str(path))
    for name in task['data_paths']:
        if not Path(name).is_file():raise ValueError('Actual input data unavailable: '+name)
        expected=task.get('data_hashes',{}).get(name)
        if expected and sha256(name)!=expected:raise ValueError('Actual data hash changed: '+name)


def choose_task(tasks,state):
    finished={'held','failed','reviewed_pending_integration'}
    return next((task for task in tasks if state['tasks'].get(task['id'],{}).get('status') not in finished),None)


def review_outcome(result):
    if result.get('admitted') is not False:raise ValueError('Agent cannot grant live admission')
    return 'reviewed_pending_integration' if all(result.get(k) is True for k in
        ('source_reviewed','implementation_ready','ledger_checked')) and result.get('checked_trades',0)>0 else 'held'


def git(repo,*args):
    p=subprocess.run(['git','-C',str(repo),*args],text=True,capture_output=True,timeout=120)
    if p.returncode:raise ValueError('Isolated Git operation failed: '+p.stderr[-1000:])
    return p.stdout.strip()


def prepare_workspace(repo,worktree_root,task):
    root=Path(worktree_root).resolve();root.mkdir(parents=True,exist_ok=True)
    workspace=root/task['id']
    if workspace.exists():
        if not (workspace/'.git').exists():raise ValueError('Existing path is not a candidate worktree')
        return workspace
    branch='continuous/'+task['id']
    git(repo,'worktree','add','-b',branch,str(workspace),'HEAD')
    p=subprocess.run(['sh','scripts/install-hooks.sh'],cwd=workspace,capture_output=True,text=True,timeout=30)
    if p.returncode:raise ValueError('Candidate hooks could not be installed')
    return workspace


def candidate_snapshot(workspace):
    names=set(git(workspace,'diff','--name-only','HEAD').splitlines())
    names.update(git(workspace,'ls-files','--others','--exclude-standard').splitlines())
    result={}
    for name in sorted(names):
        path=Path(workspace)/name
        if not allowed_file(name) or not path.is_file() or path.is_symlink():
            raise ValueError('Candidate changed a prohibited path: '+name)
        if path.stat().st_size>2*1024*1024:raise ValueError('Candidate source artifact too large: '+name)
        result[name]=sha256(path)
    return result


def verify_candidate(workspace,snapshot):
    for name,expected in snapshot.items():
        path=Path(workspace)/name
        if not path.is_file() or path.is_symlink() or sha256(path)!=expected:
            raise ValueError('Candidate changed during independent review: '+name)


def make_job(task,phase,workspace,state_dir,codex,template):
    if phase not in ('convert','review'):raise ValueError('Unknown phase')
    root=Path(state_dir).resolve();job_id=task['id']+'.'+phase
    folder=root/'jobs'/job_id;folder.mkdir(parents=True,exist_ok=True,mode=0o700)
    result=folder/'result.json';schema=folder/'schema.json';prompt=folder/'prompt.txt'
    properties={key:{'type':'boolean'} for key in ('source_reviewed','implementation_ready','ledger_checked')}
    properties.update(admitted={'type':'boolean','enum':[False]},checked_trades={'type':'integer','minimum':0},
        evidence={'type':'array','items':{'type':'string'}},summary={'type':'string'},
        method_ids={'type':'array','items':{'type':'string'}},review_path={'type':'string'})
    worker.atomic_json(schema,{'type':'object','additionalProperties':False,'properties':properties,'required':list(properties)})
    context={**task,'phase':phase,'artifact_dir':str(folder),'candidate_workspace':str(workspace),
        'converter_result':str(root/'jobs'/(task['id']+'.convert')/'result.json'),
        'frozen_candidate':str(root/'jobs'/(task['id']+'.convert')/'candidate-files.json'),
        'reviewer_identity':job_id,'admitted':False}
    text=Path(template).read_text()+'\n'+json.dumps(context,indent=2)+'\n'
    if prompt.exists() and prompt.read_text()!=text:raise ValueError('Existing job prompt changed; use a new task ID')
    prompt.write_text(text);prompt.chmod(0o600)
    return {'id':job_id,'approved':True,'kind':'agent_conversion','isolated_workspace':True,
        'cwd':str(Path(workspace).resolve()),'stdin_file':str(prompt),'timeout_seconds':task.get('timeout_seconds',3600),
        'max_attempts':2,'result_file':str(result),'argv':[str(codex),'exec','-c','approval_policy="never"',
        '--sandbox','workspace-write','--json','--add-dir',str(folder),'--output-schema',str(schema),
        '--output-last-message',str(result),'-'],
        'provenance':{'author':task['author'],'book':task['book'],'method':task['scope'],'instrument':task['instrument'],
            'source_paths':task['source_paths'],'data_paths':task['data_paths'],'adaptations':task['adaptations']}}


def run_stage(task,phase,workspace,state_dir,codex):
    root=Path(state_dir);template=Path(__file__).with_name('continuous_conversion_prompt.txt')
    job=make_job(task,phase,workspace,root,codex,template);manifest=root/'manifest.json'
    worker.atomic_json(manifest,{'schema_version':1,'approved_by':'Project owner authorized continuous source conversion and validation','jobs':[job]})
    worker.run(manifest,root,max_runtime_seconds=None,memory_mb=2048,cpu_cores=2,min_available_mb=2048)
    state=worker.read_status(root)
    if state['status']=='usage_limit':raise UsageLimit('Account or rate limit reached')
    if state['status']=='stopped':raise Stopped('Stop requested')
    item=state['jobs'][job['id']]
    if item['status']!='succeeded':raise ValueError(item.get('error','Conversion job failed'))
    result=worker.validate_result(job['result_file'])
    if result.get('admitted') is not False:raise ValueError('Agent attempted live admission')
    return result


def _run(backlog,state_dir,repo,worktree_root,codex,watch=False):
    root=Path(state_dir).resolve();root.mkdir(parents=True,exist_ok=True,mode=0o700);root.chmod(0o700)
    path=root/'pipeline.json'
    state=json.loads(path.read_text()) if path.exists() else {'schema_version':1,'tasks':{},'live_admissions':0}
    state['coordinator_pid']=__import__('os').getpid()
    def save(status):
        state.update(status=status,updated_at=worker.timestamp(),live_admissions=0)
        worker.atomic_json(path,state)
    while True:
        if (root/'STOP').exists():save('stopped');return 0
        if (root/'LIMIT_REACHED.json').exists():save('usage_limit');return 0
        data=load_backlog(backlog);task=choose_task(data['tasks'],state)
        state['backlog_tasks']=len(data['tasks']);state['approved_by']=data['approved_by']
        if task is None:
            save('waiting_for_backlog' if watch else 'backlog_processed')
            if not watch:return 0
            time.sleep(5);continue
        # Rust targets and preserved review evidence accumulate across books.
        # Leave space for the live application; do not delete research to proceed.
        if available_disk_bytes(root)<20*1024**3:
            save('waiting_for_disk');time.sleep(30);continue
        key=task['id'];item=state['tasks'].setdefault(key,{'status':'queued'})
        signature=worker.digest(task)
        if item.get('specification_sha256',signature)!=signature:
            raise ValueError('Existing backlog specification changed; use a new task ID')
        item['specification_sha256']=signature;state['current_task']=key
        try:
            verify_inputs(task)
            workspace=prepare_workspace(repo,worktree_root,task);item['workspace']=str(workspace)
            if item['status'] not in ('converted','reviewing'):
                item['status']='converting';save('running')
                converted=run_stage(task,'convert',workspace,root,codex)
                frozen=candidate_snapshot(workspace)
                freeze_path=root/'jobs'/(key+'.convert')/'candidate-files.json'
                freeze_path.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
                worker.atomic_json(freeze_path,frozen)
                item.update(status='converted',conversion_summary=converted.get('summary',''),method_ids=converted.get('method_ids',[]))
                save('running')
            frozen=json.loads((root/'jobs'/(key+'.convert')/'candidate-files.json').read_text())
            verify_candidate(workspace,frozen);item['status']='reviewing';save('running')
            reviewed=run_stage(task,'review',workspace,root,codex)
            verify_candidate(workspace,frozen)
            if candidate_snapshot(workspace)!=frozen:raise ValueError('Candidate file set changed during independent review')
            item.update(status=review_outcome(reviewed),review_summary=reviewed.get('summary',''),
                method_ids=reviewed.get('method_ids',[]),checked_records=reviewed.get('checked_trades',0),
                review_result=str(root/'jobs'/(key+'.review')/'result.json'),finished_at=worker.timestamp())
        except UsageLimit as error:
            item.update(status='usage_limit',error=str(error));save('usage_limit');return 0
        except Stopped:
            save('stopped');return 0
        except (OSError,ValueError,subprocess.SubprocessError) as error:
            item.update(status='failed',error=str(error),finished_at=worker.timestamp())
        save('running')


def run(backlog,state_dir,repo,worktree_root,codex,watch=False):
    root=Path(state_dir).resolve();root.mkdir(parents=True,exist_ok=True,mode=0o700)
    with (root/'pipeline.lock').open('a') as lock:
        try:fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        except BlockingIOError:raise ValueError('Another coordinator owns this backlog state')
        try:return _run(backlog,root,repo,worktree_root,codex,watch)
        finally:fcntl.flock(lock,fcntl.LOCK_UN)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('backlog','state-dir','repo','worktree-root','codex'):
        parser.add_argument('--'+name,required=True,type=Path)
    parser.add_argument('--watch',action='store_true')
    args=parser.parse_args()
    signal.signal(signal.SIGTERM,lambda *_:worker.request_stop(args.state_dir))
    signal.signal(signal.SIGINT,lambda *_:worker.request_stop(args.state_dir))
    return run(args.backlog,args.state_dir,args.repo,args.worktree_root,args.codex,args.watch)


if __name__=='__main__':raise SystemExit(main())
