#!/usr/bin/env python3
"""Single-worker supplied US CSV importer: exact originals, strict prices, no fills.

Source directories are read-only. Current/current-and-past folders remain
separate basket memberships. No native-price mixing or historical membership
inference. All source sizes/mtimes/hashes are checked before and after copying.
"""
import argparse
import collections
import csv
import datetime as dt
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import re
import time

PROJECT=Path(__file__).resolve().parents[3]
ARTIFACT=PROJECT/'library/us-baskets'
SOURCE=Path('/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/DATABASE/GLOBAL DATABASE')
CACHE=Path('/home/server/.cache/strategy-vault/us-market-data')
COLLECTION='norgate data 8SEP2020'
HEADER=['Date','Symbol','Security Name','Open','High','Low','Close','Volume']
DAILY=['date','open','high','low','close','volume']
BASKETS=[('us_sp500','S&P 500','sp500','SPX'),('us_nasdaq100','NASDAQ 100','nasdaq100','NDX'),
         ('us_dow30','Dow Jones Industrial Average','dow30','DJI'),('us_russell2000','Russell 2000','russell2000','RUT')]
INDEX_FIELDS=['basket','ticker','path','first','last','bars','source','source_sha256','csv_sha256','membership','status','has_volume',
              'trusted_market','currency','asset_type','timeframe','adjustment_status','source_price_units_verified',
              'membership_kind','membership_source','membership_source_sha256','mapped_index',
              'coverage_start','excluded_prefix_count','original_bars','coverage_note']
MAX_BYTES=64*1024**2


class ValidationError(ValueError):
    def __init__(self,message,details): super().__init__(message);self.details=details


def digest(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as s:
        for chunk in iter(lambda:s.read(1024*1024),b''):h.update(chunk)
    return h.hexdigest()


def validate_source_path(root,relative):
    root=Path(root).resolve();p=(root/relative).resolve()
    if not p.is_relative_to(root) or not p.is_file() or p.suffix.lower()!='.csv':
        raise ValueError('Source must be an exact existing CSV inside the read-only source root')
    return p


def date_format(values):
    if not values:raise ValueError('No dated quotation records')
    if all(re.fullmatch(r'\d{4}-\d{2}-\d{2}',s) for s in values):return 'iso'
    if all(re.fullmatch(r'\d{8}',s) for s in values):return 'compact'
    for separator,suffix in [('/',''),('-','_dash')]:
        if all(re.fullmatch(r'\d{1,2}'+re.escape(separator)+r'\d{1,2}'+re.escape(separator)+r'\d{4}',s) for s in values):
            first=[int(s.split(separator)[0]) for s in values];second=[int(s.split(separator)[1]) for s in values]
            dmy=any(x>12 for x in first);mdy=any(x>12 for x in second)
            if dmy and not mdy:return 'dmy'+suffix
            if mdy and not dmy:return 'mdy'+suffix
            raise ValueError('Day/month date order ambiguous or mixed; no guessed calendar format')
    raise ValueError('Unknown or mixed date encodings')


def parse_date(s,fmt):
    if fmt=='iso' and re.fullmatch(r'\d{4}-\d{2}-\d{2}',s):return dt.date.fromisoformat(s)
    if fmt=='compact' and re.fullmatch(r'\d{8}',s):return dt.date(int(s[:4]),int(s[4:6]),int(s[6:]))
    if fmt in {'dmy','mdy','dmy_dash','mdy_dash'}:
        separator='-' if fmt.endswith('_dash') else '/'
        if re.fullmatch(r'\d{1,2}'+re.escape(separator)+r'\d{1,2}'+re.escape(separator)+r'\d{4}',s):
            a,b,y=map(int,s.split(separator));return dt.date(y,b,a) if fmt.startswith('dmy') else dt.date(y,a,b)
    raise ValueError('Record differs from the file-proven date format')


def classify_security(ticker,name):
    if (not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,80}',ticker) or ticker in {'SPY','QQQ','DIA','IWM','IVV','VOO','VTI'} or
            re.search(r'\b(?:ETF|ETN|Exchange[ -]Traded)\b',name,re.I) or re.search(r'\bIndex\s*$',name,re.I)):
        raise ValueError('Known ETF/index or unsupported symbol identity; not an individual-stock basket row')
    if not re.search(r'\b(?:Common|Ordinary|ADR|ADS|Depositary)\b',name,re.I):
        raise ValueError('Source security label does not establish a company share; needs instrument review')
    return 'individual_stock'


def validate_csv(source,ticker,output,gap_policy='disclose'):
    if gap_policy!='disclose':raise ValueError('Gaps cannot establish missing sessions without a trusted benchmark calendar; only disclosure is admitted')
    source=Path(source);output=Path(output);output.parent.mkdir(parents=True,exist_ok=True)
    errors=[];gaps=[];count=0;zero=0;positive=False;first=None;last=None;names=set();maximum_gap=0
    try:
        with source.open('r',encoding='utf-8-sig',newline='') as inp:
            reader=csv.reader(inp)
            header=next(reader,None)
            if header!=HEADER:raise ValueError('CSV header differs from the observed vendor-export schema')
            sample=list(itertools.islice(reader,256))
            if not sample or any(len(r)!=8 for r in sample):raise ValueError('Empty/malformed CSV quotation rows')
            fmt=date_format([r[0] for r in sample])
            with output.open('x',encoding='utf-8',newline='') as out:
                writer=csv.writer(out,lineterminator='\n');writer.writerow(DAILY)
                for row in itertools.chain(sample,reader):
                    count+=1
                    if count>100000:raise ValueError('Single-file daily record count exceeds bounded 100000 limit')
                    try:
                        if len(row)!=8:raise ValueError('Malformed quotation field count')
                        if row[1]!=ticker:raise ValueError('Filename/row symbol mismatch; original suffixes are not stripped')
                        classify_security(ticker,row[2]);names.add(row[2])
                        date=parse_date(row[0],fmt)
                        if date>dt.date(2026,10,13):raise ValueError('Quotation date is after retrieval date')
                        if last is not None:
                            days=(date-last).days
                            if days<=0:raise ValueError('Duplicate or decreasing source dates; no sorting/merging')
                            maximum_gap=max(maximum_gap,days)
                            if days>7:
                                gap={'previous':last.isoformat(),'next':date.isoformat(),'calendar_days':days,'record':count}
                                gaps.append(gap)
                        first=first or date;last=date
                        values=[float(x) for x in row[3:8]];o,h,l,c,v=values
                        if not all(math.isfinite(x) for x in values):raise ValueError('Nonfinite quotation field')
                        if min(o,h,l,c)<=0 or l>min(o,c) or h<max(o,c) or h<l:raise ValueError('Invalid original OHLC geometry/positive-price requirement')
                        if v<0:raise ValueError('Negative original volume')
                        zero+=v==0;positive|=v>0
                        writer.writerow([date.isoformat(),*row[3:8]])
                    except ValueError as e:
                        if len(errors)<30:errors.append(f'Record {count}: {e}')
        witness=None
        if fmt in {'dmy','mdy','dmy_dash','mdy_dash'}:
            separator='-' if fmt.endswith('_dash') else '/';token=0 if fmt.startswith('dmy') else 1
            witness=next({'raw_date':r[0],'source_record':i+1,'unambiguous_day_token':int(r[0].split(separator)[token])}
                         for i,r in enumerate(sample) if int(r[0].split(separator)[token])>12)
        details={'original_bars':count,'first':first.isoformat() if first else None,'last':last.isoformat() if last else None,
                 'date_format':fmt,'has_volume':positive,'zero_volume_bars':zero,'security_names':sorted(names),
                 'observed_calendar_gap_count':len(gaps),'maximum_calendar_gap_days':maximum_gap,'gaps':gaps[:30],
                 'date_format_evidence':{'format':fmt,'order_witness':witness,'checked_records':count,
                                         'scope':'Each admitted record matches this proven lexical date format and a valid calendar date; exact raw dates remain in the original snapshot.'}}
        if errors:raise ValidationError('; '.join(errors[:4]),{**details,'issues':errors})
        if not count:raise ValueError('No quotation bars')
        return {**details,'bars':count,'calendar_completeness':'not_independently_verified','gap_policy':gap_policy}
    except BaseException:
        output.unlink(missing_ok=True);raise


def write_json(path,value):
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    temporary=path.with_suffix(path.suffix+'.tmp');temporary.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n');temporary.replace(path)


def discover(source=SOURCE,artifact=ARTIFACT,cache=CACHE):
    source=Path(source).resolve();artifact=Path(artifact);cache=Path(cache);result=[]
    for basket,folder,family,mapped in BASKETS:
        for past in [False,True]:
            label=folder+(' Current & Past' if past else '');p=source/COLLECTION/label
            if not p.is_dir():raise ValueError('Selected coherent source collection missing entire basket: '+label)
            files=[];ignored=[]
            for e in os.scandir(p):
                if e.is_symlink() or not e.is_file(follow_symlinks=False) or Path(e.name).suffix.lower()!='.csv':ignored.append(e.name);continue
                s=e.stat(follow_symlinks=False)
                files.append({'ticker':Path(e.name).stem,'source_relative':str(Path(e.path).relative_to(source)),
                              'bytes':s.st_size,'mtime_ns':s.st_mtime_ns})
                if len(files)>20000:raise ValueError('Basket exceeds bounded 20000-source-file limit')
            files.sort(key=lambda r:r['ticker']);bid=basket+('_past' if past else '')
            membership={'schema_version':1,'id':bid,'directory':str(p.relative_to(source)),
                        'kind':'current_plus_past_snapshot' if past else 'current_snapshot',
                        'source_membership_vintage':'unverified','retrieved':'2026-10-13',
                        'symbols':[f['ticker'] for f in files]}
            membership_bytes=(json.dumps(membership,ensure_ascii=False,sort_keys=True,indent=2)+'\n').encode()
            dest=cache/'membership'/f'{bid}.json';dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(membership_bytes)
            representative=next((f for f in files if f['ticker']=='AAPL'),files[0] if files else None)
            sample=None
            if representative:
                q=source/representative['source_relative']
                with q.open('rb') as inp:
                    head=inp.read(4096);inp.seek(max(0,q.stat().st_size-4096));tail=inp.read()
                lines=head.decode('utf-8-sig').splitlines();firstrow=next(csv.reader([lines[1]]))
                lastrow=next(csv.reader([tail.decode('utf-8').splitlines()[-1]]))
                fmt=date_format([firstrow[0],lastrow[0]])
                sample={'ticker':representative['ticker'],'first':parse_date(firstrow[0],fmt).isoformat(),
                        'last':parse_date(lastrow[0],fmt).isoformat(),'date_format':fmt,'header':next(csv.reader([lines[0]])),
                        'scope':'Representative only; complete file/constituent coverage awaits serial validation.'}
            entry={'id':bid,'name':folder+(' — Current & Past' if past else ' — Current'),
                   'family':family,'universe':'current_plus_past' if past else 'current','mapped_index':mapped,
                   'membership_kind':membership['kind'],'membership_source':membership['directory'],
                   'membership_source_sha256':hashlib.sha256(membership_bytes).hexdigest(),
                   'membership_inventory_path':str(dest.relative_to(cache)),'raw_symbol_count':len(files),
                   'source_bytes':sum(f['bytes'] for f in files),'files':files,'ignored_entries':ignored,'sample':sample}
            result.append(entry);print(json.dumps({k:entry[k] for k in ['id','raw_symbol_count','source_bytes','sample']}),flush=True)
    manifest={'schema_version':1,'retrieved':'2026-10-13','source_access':'read_only','source_root':str(source),
              'selected_collection':COLLECTION,'baskets':result,'total_source_file_locations':sum(b['raw_symbol_count'] for b in result),
              'unique_raw_symbol_identities':len({f['ticker'] for b in result for f in b['files']}),
              'total_source_bytes':sum(b['source_bytes'] for b in result),
              'source_quote_authenticity':'not_independently_verified','historical_membership':'unverified'}
    write_json(artifact/'source-discovery-2026-10-13.json',manifest);write_json(cache/'source-discovery.json',manifest)
    return manifest


def stage_source(source,destination,expected):
    before=source.stat()
    if before.st_size!=expected['bytes'] or before.st_mtime_ns!=expected['mtime_ns']:raise ValueError('Source changed after directory snapshot')
    if not 0<before.st_size<=MAX_BYTES:raise ValueError('Source is empty or exceeds bounded 64MiB file limit')
    h=hashlib.sha256();count=0;destination.parent.mkdir(parents=True,exist_ok=True)
    with source.open('rb') as inp,destination.open('xb') as out:
        for chunk in iter(lambda:inp.read(1024*1024),b''):
            count+=len(chunk)
            if count>before.st_size:raise ValueError('Source grew during copy')
            h.update(chunk);out.write(chunk)
    body=h.hexdigest();after_hash=digest(source);after=source.stat()
    if count!=before.st_size or body!=after_hash or before.st_mtime_ns!=after.st_mtime_ns or before.st_size!=after.st_size:
        raise ValueError('Source externally changed during copy/hash validation')
    if digest(destination)!=body:raise ValueError('Original snapshot failed independent destination SHA-256')
    return {'source_sha256':body,'source_hash_before':body,'source_hash_after':after_hash,
            'source_mtime_before_ns':before.st_mtime_ns,'source_mtime_after_ns':after.st_mtime_ns,
            'source_bytes':count,'byte_identity_verified':True,'source_sha256_unchanged':True,'source_mtime_unchanged':True}


def export(manifest,latest,cache,artifact,complete=False):
    controls=[];rows=[]
    for b in manifest['baskets']:
        admitted=[];missing=[];firsts=[];lasts=[];totalbars=0
        membership=('Supplied current-and-past filename union; membership vintage and date-specific membership unverified.'
                    if b['universe']=='current_plus_past' else 'Supplied current filename snapshot; vintage unverified; retrospective survivor-selection bias.')
        for f in b['files']:
            r=latest.get(f['source_relative'])
            if not r or r['status']!='ready':missing.append({'ticker':f['ticker'],'status':r['status'] if r else 'pending','reason':r.get('reason') if r else None});continue
            admitted.append('basket:'+b['id']+':'+f['ticker']);firsts.append(r['first']);lasts.append(r['last']);totalbars+=r['bars']
            row={'basket':b['id'],'ticker':f['ticker'],'path':r['path'],'first':r['first'],'last':r['last'],'bars':r['bars'],
                 'source':COLLECTION+'/'+str(Path(f['source_relative']).relative_to(COLLECTION)),
                 'source_sha256':r['source_sha256'],'csv_sha256':r['csv_sha256'],'membership':membership,
                 'status':'ready','has_volume':'true' if r['has_volume'] else 'false','trusted_market':'us','currency':'USD',
                 'asset_type':'individual_stock','timeframe':'D','adjustment_status':'not_independently_verified','source_price_units_verified':'false',
                 'membership_kind':b['membership_kind'],'membership_source':b['membership_source'],
                 'membership_source_sha256':b['membership_source_sha256'],'mapped_index':b['mapped_index'],
                 'coverage_start':r['first'],'excluded_prefix_count':0,'original_bars':r['bars'],
                 'coverage_note':'All source rows admitted unchanged in sequence; no source repairs, missing-bar fills, tail selection or independent vendor-price/adjustment certification.'}
            rows.append(row)
        quote_cutoff=max(lasts) if lasts else None
        controls.append({'id':b['id'],'name':b['name'],'family':b['family'],'universe':b['universe'],
                         'market':'us','currency':'USD','price_status':'ready' if len(admitted)==b['raw_symbol_count'] else 'partial' if admitted else 'missing',
                         'symbols':admitted,'source_counts':{'expected':b['raw_symbol_count'],'covered':len(admitted),'withheld_or_pending':len(missing)},
                         'quote_cutoff':quote_cutoff,'quote_first':min(firsts) if firsts else None,
                         'constituent_end_date_counts':dict(collections.Counter(lasts)),'bars':totalbars,
                         'membership_kind':b['membership_kind'],'membership_vintage':'unverified','historical_membership':'unverified',
                         'source_collection':COLLECTION,'mapped_index':b['mapped_index'],'adjustment_status':'not_independently_verified',
                         'source_price_units_verified':False,'missing':missing,
                         'note':f'Covered {len(admitted)} of {b["raw_symbol_count"]} supplied source identities. Actual quotation cutoff {quote_cutoff or "unavailable"}; this is 2020-era stock data, not 2026 stock data. '+membership+' Independent-symbol histories only; no portfolio capital statistics. Source adjustments and dollar price units are not independently certified.'})
    index=cache/'index.csv';temporary=index.with_suffix('.csv.tmp')
    with temporary.open('w',newline='') as out:
        writer=csv.DictWriter(out,fieldnames=INDEX_FIELDS);writer.writeheader();writer.writerows(rows)
    temporary.replace(index)
    write_json(cache/'baskets.json',{'schema_version':1,'complete':complete,'baskets':controls,'selected_collection':COLLECTION})
    write_json(artifact/'baskets-us.json',controls)
    summary={'complete':complete,'source_access':'read_only','workers':1,'selected_collection':COLLECTION,
             'source_file_locations':manifest['total_source_file_locations'],'unique_raw_symbol_identities':manifest['unique_raw_symbol_identities'],
             'processed_source_locations':len(latest),'statuses':dict(collections.Counter(r['status'] for r in latest.values())),
             'admitted_basket_rows':len(rows),'unique_normalized_CSVs':len({r['path'] for r in rows}),
             'total_admitted_basket_bars':sum(r['bars'] for r in rows),
             'total_parsed_original_bars':sum(r.get('original_bars',0) for r in latest.values()),
             'source_locations_without_parsed_bar_count':sum('original_bars' not in r for r in latest.values()),
             'date_format_counts':dict(collections.Counter(r.get('date_format','unverified') for r in latest.values())),
             'membership_warning':'Current-and-past unions do not establish historical membership; current lists have survivor-selection limitations.',
             'baskets':[{k:c[k] for k in ['id','price_status','source_counts','quote_first','quote_cutoff','bars']} for c in controls]}
    write_json(cache/'import-summary.json',summary);write_json(artifact/'import-summary-2026-10-13.json',summary)


def run(manifest,source=SOURCE,cache=CACHE,artifact=ARTIFACT,gap_policy='disclose',limit=None,retry_date_formats=False):
    source=Path(source).resolve();cache=Path(cache).resolve();artifact=Path(artifact);cache.mkdir(parents=True,exist_ok=True)
    if cache.is_relative_to(source) or source.is_relative_to(cache):raise ValueError('Cache must be separate from read-only source')
    if manifest['total_source_bytes']>16*1024**3:raise ValueError('Source scope exceeds 16GiB reviewed payload budget')
    if os.statvfs(cache).f_bavail*os.statvfs(cache).f_frsize<2*manifest['total_source_bytes']+10*1024**3:raise ValueError('Insufficient private cache reserve')
    log=cache/'import.jsonl';prior=list(map(json.loads,log.read_text().splitlines())) if log.exists() else [];latest={r['source_relative']:r for r in prior}
    jobs={f['source_relative']:f for b in manifest['baskets'] for f in b['files']};start=time.monotonic();processed=0;last_export=start
    for relative,f in jobs.items():
        if relative in latest and not (retry_date_formats and latest[relative]['status']=='withheld' and
                                       ('date encodings' in latest[relative].get('reason','') or 'date order' in latest[relative].get('reason',''))):continue
        if limit is not None and processed>=limit:break
        key=hashlib.sha256(relative.encode()).hexdigest();temp=cache/'staging'/f'{key}.csv';normal=cache/'staging'/f'{key}-daily.csv'
        temp.unlink(missing_ok=True);normal.unlink(missing_ok=True);rec={**f,'retrieved':'2026-10-13','gap_policy':gap_policy}
        try:
            original=validate_source_path(source,relative);proof=stage_source(original,temp,f);rec.update(proof)
            raw=cache/'originals'/f'{proof["source_sha256"]}.csv';raw.parent.mkdir(parents=True,exist_ok=True)
            if raw.exists():
                if digest(raw)!=proof['source_sha256']:raise ValueError('Existing original snapshot changed; refusing overwrite')
                temp.unlink()
            else:temp.replace(raw)
            rec['original_copy_path']=str(raw.relative_to(cache))
            rec.update(validate_csv(raw,f['ticker'],normal,gap_policy))
            h=digest(normal);target=cache/'daily'/f'{h}.csv';target.parent.mkdir(parents=True,exist_ok=True)
            if target.exists():
                if digest(target)!=h:raise ValueError('Existing admitted CSV changed; refusing overwrite')
                normal.unlink()
            else:normal.replace(target)
            rec.update(status='ready',path=str(target.relative_to(cache)),csv_sha256=h,
                       trusted_market='us',currency='USD',asset_type='individual_stock',timeframe='D',
                       adjustment_status='not_independently_verified',source_price_units_verified=False)
        except (ValueError,OSError,UnicodeDecodeError,csv.Error) as e:
            if isinstance(e,ValidationError):rec.update(e.details)
            rec.update(status='withheld',reason=str(e));temp.unlink(missing_ok=True);normal.unlink(missing_ok=True)
        latest[relative]=rec
        with log.open('a') as out:out.write(json.dumps(rec,ensure_ascii=False)+'\n');out.flush()
        processed+=1
        if processed%50==0:
            if processed%500==0 or time.monotonic()-last_export>60:
                export(manifest,latest,cache,artifact);last_export=time.monotonic()
            print(json.dumps({'processed':len(latest),'of':len(jobs),'statuses':dict(collections.Counter(r['status'] for r in latest.values())),
                              'elapsed_seconds':round(time.monotonic()-start,2),'last':relative}),flush=True)
    export(manifest,latest,cache,artifact,complete=len(latest)==len(jobs))
    print(json.dumps({'done':True,'processed':len(latest),'of':len(jobs),'statuses':dict(collections.Counter(r['status'] for r in latest.values()))}),flush=True)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--discover',action='store_true');parser.add_argument('--import-all',action='store_true');parser.add_argument('--limit',type=int);parser.add_argument('--retry-date-formats',action='store_true');parser.add_argument('--gap-policy',choices=['disclose'],default='disclose');args=parser.parse_args()
    if args.discover:discover()
    if args.import_all:run(json.loads((ARTIFACT/'source-discovery-2026-10-13.json').read_text()),limit=args.limit,gap_policy=args.gap_policy,retry_date_formats=args.retry_date_formats)
