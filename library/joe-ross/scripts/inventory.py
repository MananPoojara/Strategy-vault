from pathlib import Path
import hashlib,json,shutil,subprocess,os
root=Path.cwd();folder=root/'Books/Joe Ross';out=root/'library/joe-ross';cache=Path.home()/'.cache/strategy-vault/joe-ross';cache.mkdir(parents=True,exist_ok=True)
seen={};items=[]
for n,p in enumerate(sorted(folder.rglob('*'))):
 if not p.is_file() or p.suffix.lower()!='.pdf':continue
 h=hashlib.sha256()
 with p.open('rb') as stream:
  for chunk in iter(lambda:stream.read(1024*1024),b''):h.update(chunk)
 digest=h.hexdigest();rec={'source':str(p.relative_to(root)),'sha256':digest,'bytes':p.stat().st_size}
 if digest not in seen:
  dest=cache/(digest+'.pdf');text=cache/(digest+'.txt')
  if not dest.exists():shutil.copyfile(p,dest)
  meta=subprocess.run(['pdfinfo',str(dest)],capture_output=True,text=True)
  info={k.strip():v.strip() for l in meta.stdout.splitlines() if ':' in l for k,v in [l.split(':',1)]}
  extraction=subprocess.run(['pdftotext','-layout',str(dest),str(text)],capture_output=True,text=True)
  data=text.read_text(errors='replace') if text.exists() else '';pages=data.split('\f');nonempty=sum(len(t.strip())>40 for t in pages)
  if extraction.returncode:rec['extraction_error']=extraction.stderr[:800]
  rec.update(pages=int(info.get('Pages','0')),text_pages=nonempty,characters=len(data),text_file=str(text),title=info.get('Title',''),author=info.get('Author',''))
  seen[digest]=rec
  print(json.dumps({'name':p.name,'pages':rec['pages'],'text_pages':nonempty,'chars':len(data)}),flush=True)
 else:rec.update(duplicate_of=seen[digest]['source'])
 items.append(rec)
 (out/'source-inventory.json').write_text(json.dumps(items,indent=2)+'\n')
(out/'source-inventory.json').write_text(json.dumps(items,indent=2)+'\n')
print(json.dumps({'files':len(items),'unique':len(seen),'pages':sum(v['pages'] for v in seen.values()),'bytes':sum(v['bytes'] for v in seen.values())}),flush=True)
