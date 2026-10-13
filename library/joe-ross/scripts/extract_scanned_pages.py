from pathlib import Path
import json,subprocess,os,time,concurrent.futures
root=Path.cwd();inventory=root/'library/joe-ross/source-inventory.json';cache=Path.home()/'.cache/strategy-vault/joe-ross';tools=cache/'ocr-tools/root';env=os.environ.copy();env.update(LD_LIBRARY_PATH=str(tools/'usr/lib/x86_64-linux-gnu'),TESSDATA_PREFIX=str(tools/'usr/share/tesseract-ocr/4.00/tessdata'),OMP_THREAD_LIMIT='1')
records=[r for r in json.loads(inventory.read_text()) if 'duplicate_of' not in r and r['pages']>0]
# Complete the core Ross Hook book first, then the other scanned books.
records.sort(key=lambda r:(0 if 'Trading the Ross Hook' in r['source'] else 1,r['pages']))
completed=0;total=0;start=time.monotonic();jobs=[]
for r in records:
 pages=Path(r['text_file']).read_text(errors='replace').split('\f')
 for i in range(r['pages']):
  content=pages[i] if i<len(pages) else ''
  if len(content.strip())<250:jobs.append((r,i+1))
total=len(jobs);print(f'OCR candidates: {total}; 2 workers, one thread each, nice priority.',flush=True)
def process(job):
 r,page=job;output=cache/'ocr'/r['sha256'];output.mkdir(parents=True,exist_ok=True);text=output/f'{page:04}.txt'
 if text.exists():return (r['sha256'],page,'cached')
 raster=output/f'{page:04}'
 try:
  subprocess.run(['pdftoppm','-f',str(page),'-l',str(page),'-singlefile','-scale-to','1900','-png',str(cache/(r['sha256']+'.pdf')),str(raster)],check=True,capture_output=True,timeout=90)
  proc=subprocess.run([str(tools/'usr/bin/tesseract'),str(raster)+'.png','stdout','-l','eng','--psm','3'],env=env,capture_output=True,text=True,timeout=90)
  if proc.returncode:raise RuntimeError(proc.stderr)
  text.write_text(proc.stdout);return(r['sha256'],page,'ocr')
 except Exception as e:
  (output/f'{page:04}.error').write_text(str(e));return(r['sha256'],page,'error')
 finally:Path(str(raster)+'.png').unlink(missing_ok=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
 for result in pool.map(process,jobs):
  completed+=1
  if completed%25==0 or completed==total:
   progress={'completed':completed,'total':total,'seconds':round(time.monotonic()-start),'last':result}
   (cache/'ocr-progress.json').write_text(json.dumps(progress));print(json.dumps(progress),flush=True)
# Merge native text and OCR with page separators, recording what was OCR.
coverage=[]
for r in records:
 original=Path(r['text_file']).read_text(errors='replace').split('\f');pages=[];ocr_pages=[];failed=[]
 for i in range(r['pages']):
  p=cache/'ocr'/r['sha256']/f'{i+1:04}.txt';native=original[i] if i<len(original) else ''
  # Tesseract emits a terminal form feed for a single image. Strip it before
  # adding our own separator so split('\f')[page-1] remains the PDF page.
  if p.exists():pages.append(p.read_text().replace('\f','').rstrip());ocr_pages.append(i+1)
  else:
   pages.append(native)
   if len(native.strip())<250:failed.append(i+1)
 target=cache/(r['sha256']+'.review.txt');target.write_text('\f'.join(pages))
 coverage.append({'source':r['source'],'sha256':r['sha256'],'pages':r['pages'],'ocr_pages':ocr_pages,'unprocessed_short_pages':failed,'review_text':str(target)})
(root/'library/joe-ross/text-coverage.json').write_text(json.dumps(coverage,indent=2)+'\n')
print('OCR and page coverage complete.',flush=True)
