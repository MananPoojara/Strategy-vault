"""Independent Decimal rule/ledger check on actual basket constituents.

This is bounded execution parity for the declared sequential convention, not
reproduction of Connors' pooled US-stock study or a quote-adjustment certificate.
The original PDF and native CSV inputs are read only.
"""
import csv
import hashlib
import http.cookiejar
import json
from pathlib import Path
from decimal import Decimal
import urllib.parse
import urllib.request
from verify_foundation import expected_trades


def main():
    root=Path(__file__).resolve().parents[3]
    runtime=Path.home()/'.cache/strategy-vault-webapp'
    market=Path.home()/'.cache/strategy-vault/market-data'
    cfg=dict(line.split('=',1) for line in (runtime/'.env').read_text().splitlines() if line and not line.startswith('#'))
    base=cfg['VAULT_PUBLIC_URL']
    opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    request=urllib.request.Request(base+'/api/auth/sign-in/email',data=json.dumps(dict(email=cfg['VAULT_ADMIN_EMAIL'],password=cfg['VAULT_ADMIN_PASSWORD'])).encode(),headers={'Content-Type':'application/json','Origin':base},method='POST')
    with opener.open(request,timeout=20) as response: assert response.status==200
    records=list(csv.DictReader((market/'index.csv').open()))
    report=dict(date='2026-10-13',strategy='f1a',original_study_reproduced=False,all_connors_strategies_certified=False,
                rule_source='Swing Trading College IX Session1 printed/PDF13–17: C > 200MA, 5 Day Hold',
                execution_convention='Single position; entry at qualifying close; exit after five subsequent observed bars; no reentry on exit bar; unresolved terminal trade stays open.',
                limitations=['Pooled US-stock observations and original membership/adjustments/overlap policy are not reproduced.','Indian constituent prices come from the supplied snapshot; external quote authenticity and corporate actions are not certified.'],checks=[])
    for symbol in ['RELIANCE','HDFCBANK','TCS']:
        meta=next(r for r in records if r['basket']=='nifty50' and r['ticker']==symbol and r['status']=='ready')
        path=(market/meta['path']).resolve();assert path.is_relative_to(market.resolve())
        before=path.stat();data=path.read_bytes();assert hashlib.sha256(data).hexdigest()==meta['csv_sha256']
        rows=[{'Date':r['date'],'Close':r['close']} for r in csv.DictReader(data.decode().splitlines())]
        assert len(rows)==int(meta['bars']) and all(rows[i]['Date']<rows[i+1]['Date'] for i in range(len(rows)-1))
        expected,dates=expected_trades(rows)
        query=urllib.parse.urlencode({'id':'f1a','ticker':'basket:nifty50:'+symbol,'from':'2020-01-01','to':'2025-12-31','cost':2,'slippage':3,'p.trend':200,'p.hold':5})
        with opener.open(base+'/api/in/guru/connors/run?'+query,timeout=30) as response: actual=json.load(response)
        assert actual['dates']==dates and len(actual['trades'])==len(expected)
        closed=[]
        for n,(trade,oracle) in enumerate(zip(actual['trades'],expected)):
            for key in ['entry','exit','open','bars']: assert trade[key]==oracle[key],(symbol,n,key)
            assert trade['dir']==1 and trade['entry_kind']=='close' and len(trade['legs'])==1
            assert abs(Decimal(str(trade['legs'][0][1]))-oracle['entry_price'])<=Decimal('0.000001')
            assert abs(Decimal(str(trade['exit_px']))-oracle['exit_price'])<=Decimal('0.000001')
            net=oracle['return_value']-Decimal('0.001')
            assert abs(Decimal(str(trade['ret']))-net)<=Decimal('0.0000006')
            if not oracle['open']:closed.append(net)
        total=sum(closed);mean=total/len(closed)
        assert abs(Decimal(str(actual['stats']['net_profit']))-total)<=Decimal('0.0000006')
        assert abs(Decimal(str(actual['stats']['avg_net_pnl']))-mean)<=Decimal('0.0000006')
        after=path.stat();assert (before.st_size,before.st_mtime_ns)==(after.st_size,after.st_mtime_ns)
        report['checks'].append(dict(symbol=symbol,ticker='basket:nifty50:'+symbol,first=dates[0],last=dates[-1],csv_sha256=meta['csv_sha256'],source_unchanged=True,trade_records_matched=len(expected),closed_trades=len(closed),mismatches=0,closed_return_sum=str(total),mean_closed_return=str(mean)))
    output=root/'library/connors/NATIVE-FOUNDATION-CHECK-2026-10-13.json'
    output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({c['symbol']:c['trade_records_matched'] for c in report['checks']}))


if __name__=='__main__':main()
