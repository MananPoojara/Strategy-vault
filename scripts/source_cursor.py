"""Private, source-SHA coverage records. Accepted coverage never grants admission.

Only independently agreed sidecars create residual proposals. This module does
not edit a backlog, reset worker markers or infer rules from a contents title.
"""
import copy
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import subprocess
import tempfile

VERSION=1
MAX_PAGES=10000
MAX_SECTIONS=2048
MAX_EVIDENCE=256
MAX_EVIDENCE_BYTES=32*1024*1024
MAX_PROPOSALS=3
MAX_PROPOSAL_PAGES=32
ID=re.compile(r'[A-Za-z0-9][A-Za-z0-9_.-]{0,95}\Z')
SHA=re.compile(r'[0-9a-f]{64}\Z')


def require(ok,message):
    if not ok:raise ValueError(message)


def valid_id(value):
    return isinstance(value,str) and ID.fullmatch(value) is not None


def valid_sha(value):
    return isinstance(value,str) and SHA.fullmatch(value) is not None


def digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()


def file_sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
    return h.hexdigest()


def load_json(path,limit=4*1024*1024):
    p=Path(path)
    require(p.is_file() and not p.is_symlink() and p.stat().st_size<=limit,'Missing, unsafe or oversized coverage artifact')
    try:value=json.loads(p.read_text())
    except (UnicodeError,json.JSONDecodeError) as e:raise ValueError('Invalid coverage JSON') from e
    require(isinstance(value,dict),'Coverage artifact must be an object')
    return value


def atomic_json(path,value):
    p=Path(path);p.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
    fd,tmp=tempfile.mkstemp(prefix='.'+p.name+'-',dir=p.parent)
    try:
        with os.fdopen(fd,'w') as f:
            json.dump(value,f,sort_keys=True,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.chmod(tmp,0o600);os.replace(tmp,p)
        fd=os.open(p.parent,os.O_RDONLY|os.O_DIRECTORY)
        try:os.fsync(fd)
        finally:os.close(fd)
    finally:
        if os.path.exists(tmp):os.unlink(tmp)


def job_folder(task,root,phase):
    require(phase in ('convert','review') and valid_id(task.get('id')),'Unsafe coverage job identity')
    base=Path(root).resolve();folder=base/'jobs'/(task['id']+'.'+phase)
    require(folder.resolve().is_relative_to(base) and not folder.is_symlink() and not folder.parent.is_symlink(),'Unsafe private job directory')
    return folder


def _pdf_limits():
    resource.setrlimit(resource.RLIMIT_AS,(512*1024**2,512*1024**2))
    resource.setrlimit(resource.RLIMIT_CPU,(10,10))
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))


def source_identity(task):
    require(isinstance(task.get('source_paths'),list) and all(isinstance(p,str) for p in task['source_paths']),'Invalid source paths')
    pdfs=[p for p in task['source_paths'] if Path(p).suffix.lower()=='.pdf']
    require(len(pdfs)==1,'Coverage v1 requires exactly one actual PDF source')
    path=Path(pdfs[0]);expected=task['source_hashes'].get(str(path))
    require(path.is_absolute() and path.is_file() and valid_sha(expected),'Invalid PDF source identity')
    require(file_sha(path)==expected,'PDF source hash mismatch')
    try:
        p=subprocess.run(['nice','-n','10','pdfinfo',str(path)],capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=20,preexec_fn=_pdf_limits)
    except (OSError,subprocess.SubprocessError) as e:raise ValueError('Actual PDF page count unavailable') from e
    match=re.search(r'^Pages:\s+(\d+)\s*$',p.stdout,re.M)
    require(p.returncode==0 and bool(match),'Actual PDF page count unavailable')
    pages=int(match[1]);require(0<pages<=MAX_PAGES,'Actual PDF exceeds bounded coverage page limit')
    return {'path':str(path),'sha256':expected,'physical_pages':pages}


def page_cells(value,total,allow_empty=False):
    require(isinstance(value,list) and len(value)<=MAX_PAGES*2 and (allow_empty or bool(value)),'Invalid physical page cells')
    occupied=set();clean=[]
    for cell in value:
        require(isinstance(cell,dict) and set(cell)=={'page','region'},'Invalid physical page cell')
        page=cell['page'];region=cell['region']
        require(type(page) is int and 1<=page<=total and region in ('whole','left','right'),'Physical page out of bounds or invalid region')
        parts={(page,r) for r in ('left','right')} if region=='whole' else {(page,region)}
        require(not occupied&parts,'Duplicate or overlapping physical page cells')
        occupied.update(parts);clean.append({'page':page,'region':region})
    return sorted(clean,key=lambda c:(c['page'],c['region'])),occupied


def _strings(value,max_count,what):
    require(isinstance(value,list) and len(value)<=max_count and all(isinstance(x,str) and x and len(x)<=500 for x in value),what)
    require(len(value)==len(set(value)),'Duplicate '+what)
    return value


def _evidence_path(folder,name):
    require(isinstance(name,str) and name and not Path(name).is_absolute() and '..' not in Path(name).parts,'Unsafe evidence path')
    root=Path(folder).resolve();path=root/name
    require(path.resolve().is_relative_to(root),'Evidence escapes job artifact directory')
    part=path
    while part!=root:
        require(not part.is_symlink(),'Symlink evidence is not accepted');part=part.parent
    require(path.is_file() and 0<path.stat().st_size<=MAX_EVIDENCE_BYTES,'Missing or oversized evidence file')
    return path


def validate_sidecar(path,task,phase,identity=None):
    folder=job_folder(task,Path(path).parents[2],phase)
    require(Path(path).resolve().parent==folder.resolve(),'Coverage sidecar outside exact job directory')
    data=load_json(path);info=identity or source_identity(task)
    require(type(data.get('schema_version')) is int and data['schema_version']==VERSION and data.get('job_id')==task['id']+'.'+phase,'Coverage version or actor identity mismatch')
    coverage=data.get('coverage');require(isinstance(coverage,dict),'Missing source coverage')
    require(coverage.get('source_sha256')==info['sha256'],'Coverage source hash mismatch')
    require(type(coverage.get('physical_pages')) is int and coverage['physical_pages']==info['physical_pages'],'Actual PDF page-count mismatch')
    total=info['physical_pages'];require(coverage.get('bibliographic_completeness') in ('unverified','partial','declared_complete'),'Invalid bibliographic completeness')
    missing=_strings(coverage.get('missing_printed_pages'),1024,'missing printed-page descriptions')
    require(type(coverage.get('inventory_complete')) is bool,'Missing inventory completeness declaration')
    evidence=data.get('evidence');require(isinstance(evidence,list) and len(evidence)<=MAX_EVIDENCE,'Unbounded evidence list')
    proof={};size=0
    for item in evidence:
        require(isinstance(item,dict) and valid_id(item.get('id')) and item['id'] not in proof,'Invalid or duplicate evidence identity')
        file=_evidence_path(folder,item.get('path'));size+=file.stat().st_size
        require(size<=MAX_EVIDENCE_BYTES and valid_sha(item.get('sha256')) and file_sha(file)==item['sha256'],'Evidence size/hash mismatch')
        _,covered=page_cells(item.get('pages'),total);proof[item['id']]=covered
    sections=coverage.get('sections');require(isinstance(sections,list) and 0<len(sections)<=MAX_SECTIONS,'Invalid section inventory')
    section_ids=set();method_ids=set();occupied=set();clean=[]
    for section in sections:
        require(isinstance(section,dict) and valid_id(section.get('id')) and section['id'] not in section_ids,'Invalid or duplicate section identity')
        section_ids.add(section['id']);status=section.get('status');require(status in ('unread','read','unreadable','source_missing'),'Invalid source-section state')
        cells,area=page_cells(section.get('pages'),total,allow_empty=status=='source_missing')
        require(bool(area)==(status!='source_missing'),'Missing-source section must have no invented physical pages')
        require(not occupied&area,'Overlapping source sections');occupied.update(area)
        if status=='source_missing':require(bool(missing) and coverage['bibliographic_completeness']!='declared_complete','Missing source must remain a partial/unverified work')
        refs=_strings(section.get('evidence_ids'),MAX_EVIDENCE,'section evidence references');require(all(x in proof for x in refs),'Unknown section evidence')
        if status=='read':require(area<=set().union(*(proof[x] for x in refs)) if refs else not area,'Read section lacks page-bound evidence')
        methods=section.get('methods');require(isinstance(methods,list) and len(methods)<=256,'Invalid method inventory')
        normalized=[]
        for method in methods:
            require(isinstance(method,dict) and valid_id(method.get('id')) and method['id'] not in method_ids,'Invalid or duplicate method identity')
            method_ids.add(method['id']);require(valid_id(method.get('family_id')),'Missing method family identity')
            require(method.get('kind') in ('strategy','component','reading') and method.get('state') in ('pending','held','source_checked','reading_only'),'Invalid method/component state')
            require(status=='read','Method inventory requires a reviewed source section')
            if method['state']=='held':require(isinstance(method.get('hold_reason'),str) and bool(method['hold_reason'].strip()),'Held scope needs an exact reason')
            if method['kind']=='reading':require(method['state']=='reading_only','Reading material cannot imply a pending strategy')
            normalized.append({k:method[k] for k in ('id','family_id','kind','state','hold_reason') if k in method})
        clean.append({'id':section['id'],'pages':cells,'status':status,'methods':sorted(normalized,key=lambda m:m['id'])})
    batch_sections=_strings(coverage.get('batch_section_ids'),32,'batch section IDs');require(set(batch_sections)<=section_ids,'Unknown batch section')
    require(all(s['status']=='read' for s in clean if s['id'] in batch_sections),'Batch cannot certify an unread section')
    batch_methods=_strings(coverage.get('batch_method_ids'),3,'batch method IDs');require(set(batch_methods)<=method_ids,'Unknown batch method')
    selected={m['id'] for s in clean if s['id'] in batch_sections for m in s['methods']};require(set(batch_methods)<=selected,'Batch method outside selected sections')
    if coverage['inventory_complete']:require(occupied=={(p,r) for p in range(1,total+1) for r in ('left','right')},'Complete inventory does not cover all actual physical pages')
    normalized={'source_sha256':info['sha256'],'physical_pages':total,'bibliographic_completeness':coverage['bibliographic_completeness'],'missing_printed_pages':sorted(missing),'inventory_complete':coverage['inventory_complete'],'sections':sorted(clean,key=lambda s:(min((c['page'] for c in s['pages']),default=MAX_PAGES+1),s['id'])),'batch_section_ids':sorted(batch_sections),'batch_method_ids':sorted(batch_methods)}
    if phase=='review':require(data.get('accepted') is True and valid_sha(data.get('converter_sidecar_sha256')),'Independent reviewer did not accept a bound converter sidecar')
    return {'coverage':normalized,'sidecar_sha256':file_sha(path),'job_id':data['job_id'],'path':str(path),'converter_sidecar_sha256':data.get('converter_sidecar_sha256')}


def freeze_converter(task,root,identity=None):
    folder=job_folder(task,root,'convert');path=folder/'source-coverage.json';value=validate_sidecar(path,task,'convert',identity)
    frozen=folder/'source-coverage.frozen.json';trusted=Path(root)/'source-cursor-bindings'/(task['id']+'.json')
    fixed={'version':VERSION,'source_sha256':value['coverage']['source_sha256'],'original_sha256':value['sidecar_sha256'],'coverage_sha256':digest(value['coverage'])}
    if trusted.exists():
        binding=load_json(trusted)
        require(all(binding.get(k)==v for k,v in fixed.items()) and valid_sha(binding.get('frozen_sha256')),'Frozen converter binding changed; use a new task ID')
        require(frozen.is_file() and not frozen.is_symlink() and file_sha(frozen)==binding['frozen_sha256'],'Frozen converter artifact changed; preserve it and use a new task ID')
    else:
        atomic_json(frozen,load_json(path));binding={**fixed,'frozen_sha256':file_sha(frozen)};atomic_json(trusted,binding)
    atomic_json(folder/'source-coverage-binding.json',binding);return binding


def _registry(root):
    path=Path(root)/'source-cursors.json'
    value=load_json(path,16*1024*1024) if path.exists() else {'schema_version':VERSION,'sources':{},'tasks':{}}
    require(value.get('schema_version')==VERSION and isinstance(value.get('sources'),dict) and isinstance(value.get('tasks'),dict),'Invalid source registry')
    return value


def seed_registry(tasks,root):
    """Metadata only; absence of an accepted cursor is explicitly unproven."""
    root=Path(root);root.mkdir(parents=True,exist_ok=True)
    with (root/'source-cursors.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX);registry=_registry(root);before=digest(registry)
        for task in tasks:
            hashes=[]
            for name,h in task['source_hashes'].items():
                if Path(name).suffix.lower()!='.pdf':continue
                require(valid_sha(h),'Invalid expected source SHA')
                hashes.append(h);registry['sources'].setdefault(h,{'source_sha256':h,'coverage_status':'coverage_unproven','complete_bibliographic_work_reviewed':False,'accepted_reviews':[]})
            registry['tasks'].setdefault(task['id'],{'source_sha256':sorted(set(hashes)),'coverage_status':'coverage_unproven' if task.get('source_cursor_required') is True else 'legacy_coverage_unproven'})
        if digest(registry)!=before:atomic_json(root/'source-cursors.json',registry)


def _transition(previous,current):
    if not previous:return
    old={s['id']:s for s in previous['sections']};new={s['id']:s for s in current['sections']}
    require(set(old)<=set(new),'Source cursor cannot erase earlier sections')
    require(not previous['inventory_complete'] or current['inventory_complete'],'Source inventory completeness cannot regress')
    for key,a in old.items():
        b=new[key];require(a['pages']==b['pages'],'Accepted section bounds changed')
        require(a['status']!='read' or b['status']=='read','Read source section cannot regress')
        methods={m['id']:m for m in b['methods']}
        for method in a['methods']:
            require(method['id'] in methods,'Source cursor cannot erase a method')
            next_=methods[method['id']];require(all(method[k]==next_[k] for k in ('id','family_id','kind')),'Accepted method identity changed')
            if method['state']=='held':require(next_['state']=='held','Genuine held scope cannot be silently reopened')
            elif method['state'] in ('source_checked','reading_only'):require(next_['state']==method['state'],'Accepted method state cannot regress')


def residual_proposals(coverage,acceptance,max_proposals=MAX_PROPOSALS):
    require(type(max_proposals) is int and 0<=max_proposals<=MAX_PROPOSALS,'Invalid child proposal cap')
    out=[];oversized=[s['id'] for s in coverage['sections'] if s['status']=='unread' and len({c['page'] for c in s['pages']})>MAX_PROPOSAL_PAGES]
    def add(kind,section,methods):
        core={'kind':kind,'source_sha256':coverage['source_sha256'],'section_ids':[section['id']],'pages':section['pages'],'method_ids':methods}
        key='cursor-'+coverage['source_sha256'][:12]+'-'+digest(core)[:16]
        out.append({**core,'id':key,'status':'proposal_pending_root_review','admitted':False,'parent_acceptance':acceptance,'scope_notice':'A bounded residual research proposal, not verified rules, implementation or public admission.'})
    for section in coverage['sections']:
        if len(out)>=max_proposals:break
        if section['status']=='unread':
            if len({c['page'] for c in section['pages']})<=MAX_PROPOSAL_PAGES:add('unread_section',section,[])

        elif section['status']=='read':
            pending=[m['id'] for m in section['methods'] if m['state']=='pending']
            for start in range(0,len(pending),3):
                if len(out)>=max_proposals:break
                add('pending_methods',section,pending[start:start+3])
    return out,oversized


def accept_review(task,root,identity=None,max_proposals=MAX_PROPOSALS,expected_binding=None):
    root=Path(root);folder=job_folder(task,root,'convert');binding=load_json(folder/'source-coverage-binding.json')
    trusted=load_json(root/'source-cursor-bindings'/(task['id']+'.json'))
    require(binding==trusted and (expected_binding is None or binding==expected_binding),'Frozen coverage binding changed during review')
    require(binding.get('version')==VERSION and all(valid_sha(binding.get(k)) for k in ('source_sha256','original_sha256','frozen_sha256','coverage_sha256')),'Invalid frozen coverage binding')
    require(file_sha(folder/'source-coverage.json')==binding['original_sha256'] and file_sha(folder/'source-coverage.frozen.json')==binding['frozen_sha256'],'Converter coverage changed after freeze')
    info=identity or source_identity(task);converter=validate_sidecar(folder/'source-coverage.json',task,'convert',info);review=validate_sidecar(job_folder(task,root,'review')/'source-coverage.json',task,'review',info)
    require(converter['job_id']!=review['job_id'] and review['converter_sidecar_sha256']==binding['original_sha256'],'Independent sidecar does not bind the converter')
    require(converter['coverage']==review['coverage'] and digest(converter['coverage'])==binding['coverage_sha256'],'Independent source coverage disagreement')
    c=converter['coverage'];acceptance={'task_id':task['id'],'converter_sha256':converter['sidecar_sha256'],'reviewer_sha256':review['sidecar_sha256'],'reviewer_job_id':review['job_id']};proposals,oversized=residual_proposals(c,acceptance,max_proposals)
    result={'source_sha256':c['source_sha256'],'coverage_status':'independently_accepted_bounded_coverage','coverage':c,'unread_section_ids':[s['id'] for s in c['sections'] if s['status']=='unread'],'missing_source_section_ids':[s['id'] for s in c['sections'] if s['status']=='source_missing'],'unreadable_section_ids':[s['id'] for s in c['sections'] if s['status']=='unreadable'],'pending_method_ids':sorted(m['id'] for s in c['sections'] for m in s['methods'] if m['state']=='pending'),'blocked_method_ids':sorted(m['id'] for s in c['sections'] for m in s['methods'] if m['state']=='held'),'all_available_sections_reviewed':c['inventory_complete'] and all(s['status'] in ('read','source_missing') for s in c['sections']),'complete_bibliographic_work_reviewed':False,'proposals':proposals,'oversized_sections_needing_reviewed_subdivision':oversized}
    with (root/'source-cursors.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX);registry=_registry(root);entry=registry['sources'].get(c['source_sha256'],{});require(isinstance(entry,dict),'Invalid existing source cursor');_transition(entry.get('coverage'),c);history=entry.get('accepted_reviews',[])
        if acceptance not in history:history=history+[acceptance]
        registry['sources'][c['source_sha256']]={**result,'accepted_reviews':history};registry['tasks'][task['id']]={'source_sha256':[c['source_sha256']],'coverage_status':result['coverage_status'],'acceptance':acceptance};atomic_json(root/'source-cursors.json',registry)
        atomic_json(job_folder(task,root,'review')/'residual-proposals.json',{'schema_version':VERSION,'source_sha256':c['source_sha256'],'proposals':proposals,'not_appended_to_backlog':True})
    return result


def context_version(task,root):
    marker=Path(root)/'source-job-contexts'/(task['id']+'.json')
    if not marker.exists():return 1
    value=load_json(marker)
    require(value.get('context_version')==2 and value.get('task_sha256')==digest(task),'Pinned job context changed; use a new task ID')
    return 2


def prompt_text(task,phase,root,template,context):
    """Legacy text stays byte-identical. New opt-in contexts pin their templates."""
    root=Path(root);marker=root/'source-job-contexts'/(task['id']+'.json')
    if marker.exists():
        context_version(task,root);value=load_json(marker)
    else:
        existing=any((job_folder(task,root,p)/'prompt.txt').exists() for p in ('convert','review'))
        if task.get('source_cursor_required') is not True or existing:
            return Path(template).read_text()+'\n'+json.dumps(context,indent=2)+'\n',1
        require(type(task.get('source_cursor_contract_version',1)) is int and task.get('source_cursor_contract_version',1)==VERSION,'Unsupported source cursor contract version')
        value={'context_version':2,'source_cursor_contract_version':VERSION,'task_sha256':digest(task),'base_template':Path(template).read_text(),'cursor_template':Path(__file__).with_name('source_cursor_prompt_v1.txt').read_text()}
        value['base_sha256']=hashlib.sha256(value['base_template'].encode()).hexdigest();value['cursor_sha256']=hashlib.sha256(value['cursor_template'].encode()).hexdigest();atomic_json(marker,value)
    require(isinstance(value.get('base_template'),str) and isinstance(value.get('cursor_template'),str),'Invalid pinned template')
    require(value.get('base_sha256')==hashlib.sha256(value['base_template'].encode()).hexdigest() and value.get('cursor_sha256')==hashlib.sha256(value['cursor_template'].encode()).hexdigest(),'Pinned template hash mismatch')
    extended={**context,'source_cursor':{'schema_version':VERSION,'context_version':2,'actor_job_id':task['id']+'.'+phase,'sidecar_path':str(job_folder(task,root,phase)/'source-coverage.json'),'converter_frozen_path':str(job_folder(task,root,'convert')/'source-coverage.frozen.json'),'converter_binding_path':str(job_folder(task,root,'convert')/'source-coverage-binding.json'),'registry_path':str(root/'source-cursors.json'),'proposals_path':str(job_folder(task,root,'review')/'residual-proposals.json')}}
    return value['base_template']+'\n'+value['cursor_template']+'\n'+json.dumps(extended,indent=2)+'\n',2


def record_legacy_task(task,root):
    root=Path(root)
    with (root/'source-cursors.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX);registry=_registry(root)
        item=registry['tasks'].setdefault(task['id'],{'source_sha256':[]})
        require('acceptance' not in item,'Accepted cursor cannot become a legacy task')
        item['coverage_status']='legacy_prompt_coverage_unproven' if task.get('source_cursor_required') is True else 'legacy_coverage_unproven'
        atomic_json(root/'source-cursors.json',registry)
    return item['coverage_status']
