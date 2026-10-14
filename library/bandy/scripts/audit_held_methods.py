#!/usr/bin/env python3
"""Explain held QTS source conditions using real quotes; never simulate held trades."""
import argparse,json
from decimal import Decimal as D
from pathlib import Path
from verify_source_batch import read,sha


def main():
 ap=argparse.ArgumentParser();ap.add_argument('--manifest',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);a=ap.parse_args();cfg=json.loads(a.manifest.read_text());out=dict(held=['hbqts_stop_entry','hbqts_apz'],checks=[])
 for spec in cfg['datasets']:
  if spec['instrument']['kind']!='stock':continue
  rows=read(spec,cfg['to']);both=[];direction=[]
  for i,r in enumerate(rows):
   if i>=20 and r['date']>=cfg['from']:
    high=max(v['high'] for v in rows[i-20:i]);low=min(v['low'] for v in rows[i-10:i])
    if r['high']>high and r['low']<low:both.append(dict(bar=i,date=r['date'],entry_level=str(high),exit_level=str(low),open=str(r['open']),high=str(r['high']),low=str(r['low']),close=str(r['close'])))
   if i:
    du=r['high']-rows[i-1]['high'];dd=rows[i-1]['low']-r['low']
    fu=float(r['high'])-float(rows[i-1]['high']);fd=float(rows[i-1]['low'])-float(r['low'])
    decimal_side=1 if du>dd and du>0 else -1 if dd>du and dd>0 else 0
    binary_side=1 if fu>fd and fu>0 else -1 if fd>fu and fd>0 else 0
    if decimal_side!=binary_side:direction.append(dict(bar=i,date=r['date'],decimal_up=str(du),decimal_down=str(dd),f64_up=repr(fu),f64_down=repr(fd),decimal_side=decimal_side,f64_side=binary_side))
  out['checks'].append(dict(symbol=spec['symbol'],ohlc_path=spec['path'],ohlc_sha256=sha(spec['path']),both_stop_levels_touched_bars=len(both),both_stop_levels_touched=both,directional_predicate_precision_differences=len(direction),directional_predicate_examples=direction[:30],decision='Stop ordering is unresolved on daily bars. Directional quote predicates are precision-sensitive. APZ and stop-entry retain their authoritative calculation and admission; this source/data audit does not certify their ledgers.'))
 a.output.write_text(json.dumps(out,indent=2)+'\n')
if __name__=='__main__':main()
