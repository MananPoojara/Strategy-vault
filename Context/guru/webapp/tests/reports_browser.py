#!/usr/bin/env python3
"""Real authenticated data, exports, cost and basket UI checks; one Chrome process."""
import asyncio,json,os,re,zipfile,xml.etree.ElementTree as ET
from pathlib import Path
from urllib.parse import urlencode
from playwright.async_api import async_playwright,expect
from joe_ross_ceo import sheet_rows
APP=Path(__file__).resolve().parents[1]
config=dict(l.split('=',1) for l in (Path.home()/'.cache/strategy-vault-webapp/.env').read_text().splitlines() if l and not l.startswith('#'))
URL=os.environ.get('VAULT_TEST_URL',config['VAULT_PUBLIC_URL'])
OUT=Path.home()/'.cache/strategy-vault-webapp/browser-verification';OUT.mkdir(parents=True,exist_ok=True)
HEADERS=['Symbol','Signal','Entry Date','Entry Price','Exit Date','Exit Price','Pts','Gross P&L','Gross P&L %','Days Held','Taxes','Slippage','Net P&L','Net P&L %','MAE','MFE','Exit Reason']
async def main():
 checks=[];errors=[];original=None;original_tags=None;tagkey='rsi2_5d.in.NIFTY_50'
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  ctx=await browser.new_context(viewport={'width':1440,'height':1000});page=await ctx.new_page();page.on('pageerror',lambda e:errors.append(str(e)))
  try:
   login=await ctx.request.post(URL+'/api/auth/sign-in/email',data={'email':config['VAULT_ADMIN_EMAIL'],'password':config['VAULT_ADMIN_PASSWORD']},headers={'Origin':URL});assert login.status==200
   original=(await (await ctx.request.get(URL+'/api/app/me')).json())['settings']
   original_tags=await (await ctx.request.get(URL+'/api/app/annotations',params={'key':tagkey})).json()
   async def api(path,q):return await ctx.request.get(URL+'/api/in/guru/connors/'+path,params=q)
   q={'id':'rsi2_5d','ticker':'NIFTY 50','from':'2020-01-01','to':'2025-12-31','cost':'2','slippage':'3','rollover':'0'}
   response=await api('run',q);assert response.status==200;data=await response.json()
   same=await (await api('run',{**q,'cost':'5','slippage':'0'})).json()
   assert data['equity']==same['equity'] and [t['ret'] for t in data['trades']]==[t['ret'] for t in same['trades']]
   assert data['costs']['fees_bps']==2 and data['costs']['slippage_bps']==3
   assert data['stats']['avg_net_pnl']==data['stats']['avg_trade']
   for t in data['trades']:
    a=t['accounting'];assert abs(a['gross_ret']-a['fee_ret']-a['slippage_ret']-a['net_ret'])<3e-6
    assert a['gross_pnl'] is None and a['net_pnl'] is None and a['taxes'] is None
   for market,tk in [('in','NIFTY 50'),('us','SPX')]:
    for path in ['run','optimize']:
     r=await ctx.request.get(URL+f'/api/{market}/guru/connors/{path}',params={**q,'ticker':tk,'rollover':'1'});assert r.status==400 and 'roll' in (await r.text()).lower()
    r=await ctx.request.get(URL+f'/api/{market}/guru/connors/readiness',params={**q,'ticker':tk,'rollover':'1'});assert (await r.json())['strategies'][0]['status']=='needs_data'
   gridq={**q,'id':'f1a','g.hold':'2:4:1'}
   gridresponse=await api('optimize',gridq);assert gridresponse.status==200
   grid=await gridresponse.json();plain=await (await api('optimize',{**gridq,'cost':'5','slippage':'0'})).json();assert grid['combos']==3
   nd=len(grid['dims']);oldmetrics=16
   assert [r[:nd+oldmetrics] for r in grid['rows']]==[r[:nd+oldmetrics] for r in plain['rows']]
   assert grid['costs']['fees_bps']==2 and grid['costs']['slippage_bps']==3
   checks.append('Fees/slippage applied once; genuine gross/net reconciliation; no money/tax invention; unsupported rollover withheld on both markets')
   await page.goto(URL+'/full-variant1.html#/s/rsi2_5d?'+urlencode({'from':q['from'],'to':q['to'],'cost':'2','slippage':'3'}))
   await expect(page.locator('#tradesheet')).to_be_visible(timeout=30000)
   await page.evaluate("Promise.all(document.getAnimations().filter(a=>a.effect.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{})))")
   assert await page.locator('.metrics .metric>span').all_text_contents()==['Net Profit','Average Net P&L','No. Trades','Win Rate','Expectancy','MaxDD','Sharpe']
   assert await page.locator('.rtime').count()==0
   assert await page.locator('#period').evaluate('(e)=>e.nextElementSibling.matches("[data-selector-panel]")')
   assert await page.locator('#tradesheet th').all_text_contents()==HEADERS
   assert await page.locator('#detailed-stats').evaluate('(e)=>!!(e.compareDocumentPosition(document.getElementById("heatbox"))&Node.DOCUMENT_POSITION_FOLLOWING)')
   await page.locator('#detailed-stats summary').click();await expect(page.locator('.detailed-table')).to_be_visible()
   assert not any(re.search(r'capital|Started with|Ended with',s,re.I) for s in await page.locator('.detailed-table td:first-child').all_text_contents())
   assert await page.locator('[data-benchmark-comparison] tbody tr').count()==8
   checks.append('Exactly seven main stats; variant/basket controls follow period; detailed stats before MoM; 17 trade columns and expanded Nifty benchmark')
   # Annual means are computed independently from the actual endpoint equity.
   annual={};base=data['equity'][0];year=data['dates'][0][:4]
   for i,date in enumerate(data['dates']):
    if date[:4]!=year:annual[year]=data['equity'][i-1]/base-1;base=data['equity'][i-1];year=date[:4]
   annual[year]=data['equity'][-1]/base-1
   got=float(await page.locator('[data-annual-average]').inner_text())/100;assert abs(got-sum(annual.values())/len(annual))<.000051
   before=await page.locator('.heat2 tbody td[style]').first.get_attribute('style');await page.locator('#heat-palette').select_option('ocean');after=await page.locator('.heat2 tbody td[style]').first.get_attribute('style');assert before!=after
   await page.locator('[data-equity-year="2023"]').click();await expect(page.locator('#equity-navigator output')).to_contain_text('2023-')
   old=await page.locator('#equity-navigator output').inner_text();await page.locator('[data-range="start"]').evaluate('(e)=>{e.value=800;e.dispatchEvent(new Event("input",{bubbles:true}));}');assert await page.locator('#equity-navigator output').inner_text()!=old
   await page.locator('[data-equity-year="all"]').click()
   checks.append('Monthly Avg/Total matches actual annual mean; color palette changes; year shortcuts and draggable date range change chart viewport')
   async with page.expect_download() as pending:await page.locator('#trades-xlsx').click()
   download=await pending.value;path=OUT/'actual-trades-report.xlsx';await download.save_as(str(path));rows=sheet_rows(path)
   assert [rows[0].get(i) for i in range(17)]==HEADERS and len(rows)==len(data['trades'])+1
   for i,t in enumerate(data['trades'],1):
    assert rows[i][0]=='NIFTY 50';assert abs(rows[i][13]-t['ret'])<1e-12
    assert rows[i].get(7) is None and rows[i].get(10) is None and rows[i].get(12) is None
   with zipfile.ZipFile(path) as z:assert '123B47' in z.read('xl/styles.xml').decode().upper()
   meta={r.get(0):r.get(1) for r in sheet_rows(path,2)};assert meta['Fees (bps per side)']==2 and meta['Slippage (bps per side)']==3
   async with page.expect_download() as pending:await page.locator('[data-report-export="monthly"]').click()
   download=await pending.value;path=OUT/'actual-monthly-report.xlsx';await download.save_as(str(path));assert abs(sheet_rows(path)[-1][13]-sum(annual.values())/len(annual))<1e-12
   for kind in ['details','benchmark','equity']:
    async with page.expect_download() as pending:await page.locator(f'[data-report-export="{kind}"]').first.click()
    download=await pending.value;path=OUT/f'actual-{kind}-report.xlsx';await download.save_as(str(path));exported=sheet_rows(path)
    if kind=='equity':
     assert len(exported)==len(data['dates'])+1
     for i,date in enumerate(data['dates'],1):assert exported[i][0]==date and exported[i][1]==data['equity'][i-1] and exported[i][2]==data['gross_equity'][i-1]
    if kind=='benchmark':assert len(exported)==9 and exported[1][1]==data['stats']['total_return']
    if kind=='details':assert exported[1][1]==data['stats']['cagr'] and exported[1][2]==data['gross_stats']['cagr']
   for chart in ['c-eq','c-px','c-dd','c-yr']:
    async with page.expect_download() as pending:await page.locator(f'[data-chart-snapshot="{chart}"]').click()
    download=await pending.value;path=OUT/f'{chart}-actual.png';await download.save_as(str(path));assert path.read_bytes()[:8]==b'\x89PNG\r\n\x1a\n'
   checks.append('Actual trades/monthly styled workbooks match endpoint values and captured costs; all four chart PNG exports are valid')
   await page.locator('[data-action="tags"]').click();await page.locator('#annotation-tags').fill('Verification report');await page.locator('#annotation-ceo').check();await page.locator('#annotation-save').click();await expect(page.locator('#annotation-badges')).to_contain_text('CEO recommendation')
   await page.reload();await expect(page.locator('#annotation-badges')).to_contain_text('Verification report',timeout=30000)
   await page.locator('[data-action="share"]').click();assert await page.locator('#page-action-message').inner_text() or await page.locator('#page-action-message input').input_value()
   for theme in ['midnight','forest','sand','plum','cobalt']:
    before=await page.evaluate('getComputedStyle(document.body).backgroundColor')
    async with page.expect_response(lambda r:r.url.endswith('/api/app/settings') and r.request.method=='PUT' and r.request.post_data_json.get('theme')==theme) as savedtheme:await page.locator(f'#themes [data-theme="{theme}"]').click()
    assert (await savedtheme.value).status==200
    await expect(page.locator('#flipov')).to_have_count(0,timeout=30000)
    await expect(page.locator('#tradesheet')).to_be_visible(timeout=30000)
    assert await page.evaluate("CHARTS.every(x=>x.ch.options().layout.background.color===cv('--panel'))")
    assert await page.evaluate('getComputedStyle(document.body).backgroundColor')!=before
    assert (await (await ctx.request.get(URL+'/api/app/me')).json())['settings']['theme']==theme
   await page.locator('#themes [data-theme="midnight"]').click();await expect(page.locator('#flipov')).to_have_count(0,timeout=30000);await expect(page.locator('#tradesheet')).to_be_visible(timeout=30000)
   await page.evaluate('finishMetricAnimations()')
   await page.screenshot(path=str(OUT/'reports-midnight-desktop.png'),full_page=True)
   checks.append('Persistent research tags and administrator CEO recommendation; shareable authenticated page link; five whole-page themes persist')
   # The Page PDF action invokes native print/Save-as-PDF; produce its actual print layout through Chrome.
   await page.evaluate('()=>{window.printCalls=0;window.print=()=>window.printCalls++;}')
   await page.locator('[data-action="pdf"]').click();assert await page.evaluate('window.printCalls')==1
   await page.emulate_media(media='print');await page.pdf(path=str(OUT/'actual-whole-page-report.pdf'),format='A4',landscape=True,print_background=True)
   import subprocess
   pdfpath=str(OUT/'actual-whole-page-report.pdf');text=subprocess.check_output(['pdftotext',pdfpath,'-'],text=True);assert 'Tradesheet' in text and 'Detailed statistics' in text and 'Monthly returns' in text
   assert 'Slippage: 3 bps per side' in text
   assert f"{data['stats']['net_profit']*100:+.2f}%" in text
   assert abs(float((await page.locator('[data-tile="0"]').inner_text()).rstrip('%'))/100-data['stats']['net_profit'])<.000051
   for t in data['trades']:assert '-'.join(t['entry'].split('-')[::-1]) in text
   info=subprocess.check_output(['pdfinfo',pdfpath],text=True);assert int(re.search(r'Pages:\s+(\d+)',info).group(1))>1
   await page.emulate_media(media='screen');await page.evaluate('dispatchEvent(new Event("afterprint"))')
   checks.append('Page PDF action opens print; actual Chrome PDF contains complete report, detailed stats, monthly table and tradesheet')
   # Four baskets select actual cash symbols, not old index charts.
   controls=json.loads((APP.parent/'frontend/data/baskets.json').read_text())
   for b in controls:
    symbol=next((s for s in b['symbols'] if s.endswith(':RELIANCE')),b['symbols'][0])
    r=await api('run',{**q,'ticker':symbol,'cost':'0','slippage':'0','from':'2020-01-01'});assert r.status==200,(b['id'],await r.text());native=await r.json();assert native['provenance']['qualified_ticker']==symbol and native['ohlc']['d']==native['dates']
    assert all(t['accounting']['symbol']==symbol.split(':')[-1] for t in native['trades'])
    assert native['provenance']['quote_accuracy_independently_verified'] is False
    await page.goto(URL+'/full-variant1.html#/s/rsi2_5d?'+urlencode({'basket':b['id'],'tk':symbol,'from':'2020-01-01','to':q['to']}));await expect(page.locator('#c-eq')).to_be_visible(timeout=30000)
    await expect(page.locator('#basket-symbol')).to_have_value(symbol)
    assert await page.evaluate('STRAT.d.dates')==native['dates'] and await page.evaluate('STRAT.d.equity')==native['equity']
    await expect(page.locator('[data-results-provenance]')).to_contain_text('cash-stock')
    await expect(page.locator('[data-basket-note]')).to_contain_text(b['note'])
    await page.locator('[data-tf="W"]').click();await expect(page.locator('#pxread')).to_contain_text('Weekly candles');await page.locator('[data-tf="M"]').click();await expect(page.locator('#pxread')).to_contain_text('Monthly candles')
   async with page.expect_download() as pending:await page.locator('#trades-xlsx').click()
   download=await pending.value;path=OUT/'actual-native-trades-report.xlsx';await download.save_as(str(path));source={r.get(0):r.get(1) for r in sheet_rows(path,2)}
   assert source['Native source SHA256']==native['provenance']['source_sha256'] and source['Membership']==native['provenance']['membership']
   native_rows=sheet_rows(path)[1:];assert len(native_rows)==len(native['trades'])
   assert all(r[0]==symbol.split(':')[-1] and r[13]==t['ret'] for r,t in zip(native_rows,native['trades']))
   checks.append('All four basket selections run actual qualified stocks with OHLC/provenance, current-universe limits, real weekly/monthly candles; NSE500 partial coverage explicit')
   await page.set_viewport_size({'width':390,'height':844});await page.screenshot(path=str(OUT/'reports-native-mobile.png'),full_page=True);assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth')
   # Missing native benchmark volume or real futures rolls cannot become fabricated strategy charts.
   await page.goto(URL+'/full-variant1.html#/s/tps_stocks?'+urlencode({'basket':'nifty50','tk':'basket:nifty50:RELIANCE','from':'2000-01-03'}));await expect(page.locator('[data-backtest-state]')).to_contain_text('benchmark',timeout=30000);assert await page.locator('#c-eq').count()==0
   await page.goto(URL+'/full-variant1.html#/s/rsi2_5d?rollover=1');await expect(page.locator('[data-backtest-state]')).to_contain_text('Rollover',timeout=30000);assert await page.locator('#c-eq').count()==0
   checks.append('Mobile layout passes; missing benchmark volume and unsupported positive rollover show Data missing with no charts')
   assert not errors,errors
   report={'checks':checks,'page_errors':errors,'basket_coverage':[(b['id'],len(b['symbols'])) for b in controls],'trade_rows_verified':len(data['trades']),'quote_cutoff':'2026-10-01','pdf_flow':'Native browser print/Save as PDF, verified real Chrome print artifact'}
   (OUT/'reports-browser-report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
  except Exception:
   print(json.dumps({'page_errors':errors,'url':page.url},indent=2));await page.screenshot(path=str(OUT/'reports-failure.png'),full_page=True);raise
  finally:
   await page.goto(URL+'/admin.html')
   if original is not None:await ctx.request.put(URL+'/api/app/settings',data=original,headers={'Origin':URL})
   if original_tags is not None:await ctx.request.put(URL+'/api/app/annotations',params={'key':tagkey},data={'tags':original_tags['tags'],'ceo':original_tags['ceo']},headers={'Origin':URL})
   await ctx.close();await browser.close()
if __name__=='__main__':asyncio.run(main())
