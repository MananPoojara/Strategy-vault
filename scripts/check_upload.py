#!/usr/bin/env python3
"""Review tracked/staged files without printing matched private content."""
import argparse,json,re,subprocess
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('--staged',action='store_true');parser.add_argument('--env-file',type=Path);args=parser.parse_args()
command=['git','diff','--cached','--name-only','--diff-filter=ACMR','-z'] if args.staged else ['git','ls-files','-z']
paths=subprocess.check_output(command).decode().split('\0');findings=[];private=[]
if args.env_file:
 for line in args.env_file.read_text().splitlines():
  if '=' in line and not line.startswith('#'):
   key,value=line.split('=',1)
   if any(part in key for part in ['SECRET','TOKEN','PASSWORD']) and len(value)>=8:private.append(value)
blocked={'.pdf','.epub','.djvu','.chm','.doc','.docx','.xlsx','.xls','.csv','.tsv','.parquet','.duckdb','.sqlite','.db','.zip','.rar','.mp3','.mp4','.pem','.key'}
patterns=[r'gh[pousr]_[A-Za-z0-9]{30,}',r'github_pat_[A-Za-z0-9_]{30,}',r'AKIA[0-9A-Z]{16}',r'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----']
for filename in paths:
 if not filename:continue
 path=Path(filename)
 if not path.is_file():continue
 if path.suffix.lower() in blocked or path.name.startswith('.env') and path.name!='.env.example' or any(part in {'node_modules','target','Books','data'} for part in path.parts):findings.append({'file':filename,'reason':'Excluded private/data/build file'});continue
 if path.stat().st_size>10_000_000:findings.append({'file':filename,'reason':'Unexpectedly large source file'});continue
 try:text=path.read_text()
 except UnicodeDecodeError:continue
 if any(value in text for value in private) or any(re.search(pattern,text) for pattern in patterns):findings.append({'file':filename,'reason':'Possible secret; content redacted'})
print(json.dumps({'reviewed_files':len([p for p in paths if p]),'findings':findings},indent=2))
raise SystemExit(bool(findings))
