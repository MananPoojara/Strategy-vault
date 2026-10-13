"""Bounded, independent audit of live f1a execution; never a blanket Connors certificate.

Checks actual prices, 200-day SMA, five subsequent trading bars and sequential
single-position execution. Source table does not establish all simulation settings.
"""
import hashlib
import http.cookiejar
import json
import urllib.parse
import urllib.request
from decimal import Decimal
from pathlib import Path
import sys

sys.path.insert(0,str(Path(__file__).parents[2]/'chande/scripts'))
from verify_baseline import admitted_rows


def expected_trades(rows, first='2020-01-01', last='2025-12-31'):
    close=[Decimal(r['Close']) for r in rows]
    trend=[None if i<199 else sum(close[i-199:i+1])/200 for i in range(len(rows))]
    selected=[i for i,r in enumerate(rows) if first<=r['Date']<=last]
    assert selected
    start,end=selected[0],selected[-1]
    result=[];i=start
    while i<end:
        if trend[i] is None or close[i]<=trend[i]:
            i+=1;continue
        exit_index=min(i+5,end)
        open_trade=i+5>end
        result.append(dict(entry=rows[i]['Date'],exit=rows[exit_index]['Date'],
                           entry_price=close[i],exit_price=close[exit_index],open=open_trade,
                           return_value=close[exit_index]/close[i]-1, bars=exit_index-i))
        # This is the existing simulator's sequencing assumption: no entry on the exit bar.
        i=exit_index+1
    return result,[r['Date'] for r in rows[start:end+1]]


def main():
    root=Path(__file__).resolve().parents[3]
    runtime=Path.home()/'.cache/strategy-vault-webapp'
    config=dict(line.split('=',1) for line in (runtime/'.env').read_text().splitlines() if line and not line.startswith('#'))
    base=config['VAULT_PUBLIC_URL']
    opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    login=urllib.request.Request(base+'/api/auth/sign-in/email',
                                data=json.dumps(dict(email=config['VAULT_ADMIN_EMAIL'],password=config['VAULT_ADMIN_PASSWORD'])).encode(),
                                headers={'Content-Type':'application/json','Origin':base},method='POST')
    with opener.open(login,timeout=20) as response:
        assert response.status==200
    source=root/'Books/Larry Connors/Larry Connors - Swing Trading College IX 2010, $3999 (tradingmarkets.com)/100406_STC_Session_1_Final useful.pdf'
    report=dict(review_date='2026-10-09',strategy='f1a',source=str(source.relative_to(root)),
                source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),source_pages='Session 1, printed slides 13–17 / PDF13–17',
                rule_evidence=['Buy stocks above their 200-day SMA','Table: C > 200MA, 5 Day Hold'],
                original_study_reproduced=False,all_connors_strategies_certified=False,
                assumptions_not_established_by_this_source_table=[
                    'Single instrument and at most one open trade; source counts millions of US-stock observations.',
                    'Entry at the signal close; exit five subsequent trading bars later.',
                    'No entry on the same bar as an exit; final unresolved trade remains open.',
                    'Point-in-time stock universe, corporate actions, costs and original trade ledger are not reproduced.'],
                checks=[])
    for market,ticker,path in [('in','NIFTY 50',runtime/'data/idx_data.csv'),('us','SPY',runtime/'data/us_data.csv')]:
        before=path.stat()
        rows=admitted_rows(path,ticker)
        expected,dates=expected_trades(rows)
        query=urllib.parse.urlencode({'id':'f1a','ticker':ticker,'from':'2020-01-01','to':'2025-12-31',
                                    'cost':0,'slippage':0,'rollover':0,'p.trend':200,'p.hold':5})
        with opener.open(base+'/api/'+market+'/guru/connors/run?'+query,timeout=30) as response:
            assert response.status==200
            actual=json.load(response)
        assert actual['dates']==dates
        assert len(actual['trades'])==len(expected)
        for n,(trade,oracle) in enumerate(zip(actual['trades'],expected)):
            for key in ['entry','exit','open','bars']:
                assert trade[key]==oracle[key],(market,n,key,trade,oracle)
            assert trade['dir']==1 and trade['entry_kind']=='close' and len(trade['legs'])==1
            assert abs(Decimal(str(trade['legs'][0][1]))-oracle['entry_price'])<=Decimal('0.000001')
            assert abs(Decimal(str(trade['exit_px']))-oracle['exit_price'])<=Decimal('0.000001')
            assert abs(Decimal(str(trade['ret']))-oracle['return_value'])<=Decimal('0.0000006')
        after=path.stat()
        assert before.st_size==after.st_size and before.st_mtime_ns==after.st_mtime_ns
        report['checks'].append(dict(market=market,ticker=ticker,first=dates[0],last=dates[-1],
                                    source_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),source_unchanged=True,
                                    trades_matched=len(expected),closed_trades=sum(not t['open'] for t in expected),
                                    mismatches=0,checked=['SMA predicate','date window','entry date/price','exit date/price',
                                                          'direction','five-bar hold','open-state','zero-cost return']))
    catalog=json.loads((root/'Context/guru/webapp/verification/reference/in-catalog.json').read_text())
    books={}
    for entry in catalog:
        b=books.setdefault(entry['book'],{'catalogued_variants':0,'catalogue_status_counts':{},'full_independent_source_certification':'not established by this audit'})
        b['catalogued_variants']+=1
        status=entry['status'];b['catalogue_status_counts'][status]=b['catalogue_status_counts'].get(status,0)+1
    report['catalogue_inventory']={'basis':'Historical captured source catalogue, not live date-specific data admission', 'books':books}
    output=Path(__file__).parents[1]/'FOUNDATION-CHECK.json'
    output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({c['ticker']:c['trades_matched'] for c in report['checks']}))


if __name__=='__main__':
    main()
