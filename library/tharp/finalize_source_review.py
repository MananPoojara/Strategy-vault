#!/usr/bin/env python3
import json
from pathlib import Path
base=Path(__file__).parent
p=base/'source-review-2026-10-13.json';d=json.loads(p.read_text())
primary=[];coauthored=[]
for sid,s in d['sources'].items():
 f=s['files'][0];parts=f['source_relative'].split('/')
 if parts[0]=='Trading Books':
  row={'id':sid,'folder':parts[1],'title':s['title'],'kind':s['entry_kind_label']}
  (primary if parts[1].startswith('Van Tharp - ') else coauthored).append(row)
 s['missing']=list(dict.fromkeys([m for r in s['review']['rules'] for m in r['missing']]))
 if not s['missing']:s['missing']=['No complete standalone market-rule conversion is certified by this bounded reading scope.']
 for c in s['category_evidence']:
  c['basis']=s['summary'];c['evidence_reader_ids']=list(dict.fromkeys(e['reader_id'] for e in s['review']['evidence']))
d['reconciliation']['primary_document_folders']=primary;d['reconciliation']['coauthored_document_folders']=coauthored
assert len(primary)==11 and len(coauthored)==2
# Exact coverage of selected portions, without claiming every paragraph of this article was read.
s=d['sources']['source-4ce52264c05e7065'];s['review']['reading_limits']='Selected framework passages on PDF pages 1–8 and the notes on PDF page 9 were reviewed. The separate course referenced in the article was not fully reviewed. No complete price-trigger system is certified.'
s['review']['evidence'][0]['finding']='Selected framework passages and source notes reviewed; market selection and system parts are described without a single complete fixed signal.'
for f in d['file_reviews']:
 if f['source_title_id']=='source-4ce52264c05e7065':f['finding']='Selected system-specification passages on pages 1–8 and notes on page 9 reviewed; no complete fixed price-trigger system certified.';f['missing_rules_or_data']=s['review']['reading_limits']
# Keep display/file labels distinct from actual cover identity.
d['sources']['source-be82b8940bd02f17']['source_identity']['title']='How to Develop a Winning Trading System / Developing a Winning Trading/Investing System That Fits You'
d['sources']['source-d7e60cd2543a04a8']['source_identity']['title']='Trade Your Way to Financial Freedom'
cover_titles={
 '693120938ec50ac07841ba8d2bb660cd':'How to Use Risk to Become a More Successful Investor',
 '836b196122cf99d8159d094e2d7f0744':'How to Control Stress to Become a More Successful Investor',
 '69a0876b2ac37a0c0a2504dd31ee357f':'How to Control Losing Attitudes to Become a More Successful Investor',
 '5a3758adc6d0923e83cdf101be11a2c4':'How to Develop Discipline to Become a More Successful Investor',
 'abe833e31987a9cdec46277e16fce629':'How to Make Sound Decisions to Become a More Successful Investor',
 '79d3c0736ae36c6449b2b778bbf68501':'How to Develop a Winning Trading System / Developing a Winning Trading/Investing System That Fits You'}
for f in d['file_reviews']:
 if f['reader_id'] in cover_titles:f['actual_title']=cover_titles[f['reader_id']]

# Candidate duplicates remain hypotheses, except the matching named work and checked passage.
d['reconciliation']['copy_relationships']=[
 {'source_ids':['source-c181428e93432e57','source-d7e60cd2543a04a8'],'kind':'Same named work, different scan hashes','evidence':'Matching random-entry passage at PDF 116 and PDF 111; exact alternate edition/printing unresolved'},
 {'source_ids':['source-d310cdc2f7752e88'],'kind':'Two source files of the same named work','evidence':'Principal source copyright 2001 checked; alternate copy edition unresolved'},
 {'source_ids':['source-fbd810b90b7c6179','source-e467375eb3f9c314'],'kind':'Related newsletter/report copies and repeated article titles','evidence':'Opening-page titles and layouts suggest overlap; no complete byte/content-equivalence proof beyond exact file hashes'}]
d['implementation_audit']=[
 {'method_id':'tharp_random_entry','finding':'Former Wilder-10 calculation did not match the checked source’s EMA10. Parent has corrected it to alpha 2/11 with declared SMA seeding. Workbook EMA19 and Super Trader ATR20 are separate source models, not period variants certifying the same TYW study.','source_evidence':'TYW PDF 116 printed 200–201; Workbook PDF 66 printed 83; Super Trader PDF 162/167 printed 148/153','remaining':'Source RNG seed, fill timing, entry-bar stop convention, ten-futures contract sizing and original cost/roll model are not reproduced by a single normalized series.'},
 {'method_id':'tharp_55_21_breakout','finding':'The 55-day entry and one-way 21-day adverse trailing stop match the checked mechanical clauses. Entry-bar exit is omitted by the implementation; a daily bar reaching both levels cannot certify event order. Full-equity normalized default is not the source’s fixed-number futures contract model.','source_evidence':'TYW PDF 159 printed 286–287','remaining':'Declare execution convention or withhold ambiguous daily bars; original ten-commodity 1981–1991 portfolio is not reproduced.'},
 {'method_id':'tharp_false_breakout','finding':'Workbook PDF 60 printed 75 also permits re-entry at the original price on day one or two only. Catalogue’s abbreviated worksheet should preserve that clause. Its profit exit remains incomplete, so withholding a complete backtest is appropriate.','source_evidence':'Workbook PDF 60 printed 75','remaining':'Complete profit exit, authentic tick units and intraday execution sequence.'},
 {'method_id':'tharp_110day_breakout','finding':'Super Trader PDF 227 printed 213 discusses stocks, a 110-day breakout, weekly-volatility worst-case exit and close trail. It does not fix the volatility definition or multiplier. Existing not_testable treatment is supported.','source_evidence':'Super Trader PDF 227 printed 213','remaining':'Weekly-volatility formula/multiplier, fill and stock population rules.'}
]
p.write_text(json.dumps(d,indent=2,ensure_ascii=False)+'\n')
lines=['# Van Tharp source review — 13 October 2026','',
'The original Research share was read without changing its files. No web research was used. This is a bounded source review: the cited pages were checked, but it is not a complete review or conversion certificate for every book, workbook or newsletter. No published source performance has been certified by this review.','',
'## Reconciliation','',
'Before this review, the catalogue represented one curated work, *Trade Your Way to Financial Freedom*. The document inventory covers 20 source units and 56 files: 50 PDFs and six legacy Word documents, each with a different recorded SHA-256. There are 11 primary Van Tharp document folders in Trading Books and two coauthored document folders. Six course units and one USB-folder interview account for the remaining source units. A further Trading Books folder contains only one audio interview and is excluded from the document count.','',
'These totals must not be displayed as 20 distinct published books. Trade Your Way appears in two source folders with different scans; one newsletter folder has seventeen files; another unit contains five slide presentations; others are course attachments. Four conventional book identities were confirmed from actual title/copyright material: Trade Your Way (1999), Financial Freedom Through Electronic Day Trading (2001), Safe Strategies (2004) and Super Trader (2009). The exact total of distinct works and editions remains unresolved.','',
'Private reader-file hashes match the manifest and prior original-source/destination equality proof. Original Research byte counts were checked in this pass, but every original file was not rehashed again. PDF page numbers below identify the actual local file; printed pagination is supplied only when checked. Scanned files that expose distributor text were not treated as readable full books merely because text extraction succeeded.','',
'## The eleven primary document folders','', '| Source | Kind | Review scope |','|---|---|---|']
for row in primary:
 s=d['sources'][row['id']];lines.append(f"| {s['title']} | {s['entry_kind_label']} | {s['review']['status_label']} |")
lines+=['','The coauthored folders are **Safe Strategies for Financial Freedom** (Van K. Tharp, D. R. Barton Jr. and Steve Sjuggerud) and **The Ten Tasks of Top Trading** (Van K. Tharp and Henry O. Pruden). Financial Freedom Through Electronic Day Trading is in a Van Tharp-prefixed folder, but its actual source credit also includes **Brian June**. The Infinite Wealth manual cover credits **John Burley** with Tharp. Participant plans and guest articles retain their contributors.','', '## Per-source findings','']
for sid,s in d['sources'].items():
 lines += ['### '+s['title'],'',f"Source ID: `{sid}`. Kind: {s['entry_kind_label']}. Credit: {s['source_identity']['credit']}. Year: {s['source_identity']['year'] if s['source_identity']['year'] is not None else 'not established as one publication year'}. Categories: {', '.join(s['categories'])}.",'',s['summary'],'', '**Evidence and reading scope**','']
 for e in s['review']['evidence']:
  pages=', '.join(str(a) if a==b else f'{a}–{b}' for a,b in e['page_range']) or 'DOC paragraph text'
  lines.append(f"- `{e['file']}` (`{e['reader_id']}`), PDF pages {pages}: {e['finding']}")
 lines+=['',s['review']['reading_limits'],'']
 for r in s['review']['rules']:
  lines+=['**'+r['name']+'**','', 'Inputs: '+ '; '.join(r['inputs'])+'.','', 'Entry: '+r['entry'],'','Exit: '+r['exit'],'','Missing or separate requirements: '+ '; '.join(r['missing'])+'.','', 'Required data/model: '+ '; '.join(r['data_requirement'])+'.','']
lines+=['## Existing strategy accuracy review','', 'Mechanical source clauses, their instrument population, execution conventions and the original experiment’s accounting are separate checks. A matching indicator alone does not establish an original performance reproduction.','']
for a in d['implementation_audit']:
 lines+=['### '+a['method_id'],'',a['finding'],'','Source: '+a['source_evidence']+'.','', 'Remaining: '+a['remaining'],'']
lines+=['## Educational material and unavailable scope','',
'Psychology and money-management sources can have reading cards, source-supported summaries and conceptual outlines without strategy performance. R-multiple distributions from the trading game are simulated teaching inputs, not observed historical returns. A sizing formula is a reusable component and not a complete entry/exit system. The five long Peak Performance manuals, Infinite Wealth manual and alternate system-development course scan have only bounded visual cover/contents previews here; their bodies and audio were not certified.','',
'The audio-only file `Cash Flow 202 Psychology of winning traders by Van K. Tharp (Audio Interview).mp3` is 31,892,054 bytes. Its title and file metadata were inventoried, but it was not listened to or transcribed.','',
'All exact original source locators, record IDs, reader IDs, hashes, checked pages, actual source-credit fields and limitations are in `source-review-2026-10-13.json` under `file_reviews`. No complete conversion or verified performance flag is set.']
(base/'source-review-2026-10-13.md').write_text('\n'.join(lines)+'\n')
print('Finalized JSON and Markdown, 20 source units / 56 file reviews / 11 primary folders.')
