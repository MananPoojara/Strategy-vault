"""Read-only Tharp source reconciliation and serial bounded PDF extraction."""
import hashlib,json,re,subprocess
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=Path(__file__).resolve().parent
PRIVATE=Path.home()/'.cache/strategy-vault-webapp'
RESEARCH=Path('/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=research')

def digest(path):
    h=hashlib.sha256()
    with path.open('rb') as handle:
        for block in iter(lambda:handle.read(1024*1024),b''):h.update(block)
    return h.hexdigest()

def main():
    coverage=json.loads((ROOT/'library/source-vault/book-coverage.json').read_text())['authors']['tharp']
    manifest=json.loads((PRIVATE/'book-index.json').read_text())['files']
    readers={v['path']:(key,v) for key,v in manifest.items()}
    record={'review_date':'2026-10-13','internet_used':False,'source_mutations':False,'scope':'Inventory/byte identity and bounded extracts; summaries and complete-rule review remain separate.','source_units':[]}
    for unit in coverage['source_titles']:
        item={'source_title_id':unit['id'],'inventory_title':unit['title'],'metadata_credit_unverified':unit['author_metadata'],'files':[]}
        for f in unit['files']:
            copy=f['verified_local_copy']; relative=copy['path'].removeprefix('Books/'); key,reader=readers[relative]
            path=(PRIVATE/'books'/relative).resolve()
            if not path.is_relative_to((PRIVATE/'books').resolve()):raise ValueError('Cache escape')
            actual=digest(path)
            if actual!=copy['sha256'] or actual!=reader['sha256']:raise ValueError('Source/cache hash disagreement')
            source=RESEARCH/f['source_relative']
            try:
                info=source.stat(); source_status='exists_byte_count_matches' if info.st_size==path.stat().st_size else 'byte_count_disagrees'
            except OSError:source_status='currently_unavailable'
            evidence={'record_id':f['record_id'],'source_relative':f['source_relative'],'sha256':actual,'bytes':path.stat().st_size,'reader_id':key,'reader_path':relative,'format':reader['format'],'source_readonly_status':source_status,'byte_equality_basis':'Current staged hash matches reader manifest and previous Research-source/destination SHA equality proof; current Research file metadata checked without rewriting or reconverting it.'}
            if reader['format']=='pdf':
                info=subprocess.run(['pdfinfo',str(path)],capture_output=True,text=True,timeout=30)
                match=re.search(r'^Pages:\s+(\d+)',info.stdout,re.M)
                if info.returncode or not match:
                    evidence['extract_status']='pdf_metadata_unreadable'
                else:
                    count=int(match.group(1)); last=min(count,20)
                    result=subprocess.run(['pdftotext','-f','1','-l',str(last),'-layout',str(path),'-'],capture_output=True,text=True,timeout=60)
                    text=result.stdout;file=OUT/'extracts'/f'{key}-first.txt';file.write_text(text)
                    evidence.update(pages=count,extracted_ranges=[[1,last]],extract_path=str(file.relative_to(OUT)),extract_chars=len(text),extract_status='text_layer_available' if len(re.sub(r'\s','',text))>50 else 'scan_or_no_text_layer',title_head=text.strip()[:700])
            else:evidence['extract_status']='legacy_doc_reader_not_available_yet'
            item['files'].append(evidence)
        record['source_units'].append(item)
        print(json.dumps({'source_title_id':unit['id'],'title':unit['title'],'files':len(item['files']),'extraction':[{'reader_id':f['reader_id'],'pages':f.get('pages'),'chars':f.get('extract_chars'),'status':f['extract_status']}for f in item['files']]}),flush=True)
    (OUT/'source-reconciliation-2026-10-13.json').write_text(json.dumps(record,indent=2)+'\n')

if __name__=='__main__':main()
