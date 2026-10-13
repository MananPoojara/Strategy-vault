#!/usr/bin/env python3
"""Real API and browser regression for honest backtest availability; no market data is mocked."""
import asyncio,json,os
from pathlib import Path
from legacy_response import project_legacy
from playwright.async_api import async_playwright,expect
APP=Path(__file__).resolve().parents[1]
ENV=Path(os.environ.get('VAULT_ENV_FILE',Path.home()/'.cache/strategy-vault-webapp/.env'))
config=dict(line.split('=',1) for line in ENV.read_text().splitlines() if line and not line.startswith('#'))
URL=os.environ.get('VAULT_TEST_URL',config['VAULT_PUBLIC_URL'])
OUT=Path.home()/'.cache/strategy-vault-webapp/browser-verification';OUT.mkdir(parents=True,exist_ok=True)
OPTIONS=['oiss_short_put','oiss_short_call','oiss_plus_three_put','oiss_plus_three_call','otvt_cvr1_sell_puts','otvt_cvr1_buy_puts','otvt_diz_short_strangle','otvt_sector_options_overpriced']
async def main():
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  ctx=await browser.new_context(viewport={'width':1440,'height':1000})
  page=await ctx.new_page();errors=[];checks=[];states=[];original=None
  page.on('pageerror',lambda e:errors.append(str(e)))
  try:
   login=await ctx.request.post(URL+'/api/auth/sign-in/email',data={'email':config['VAULT_ADMIN_EMAIL'],'password':config['VAULT_ADMIN_PASSWORD']},headers={'Origin':URL})
   assert login.status==200
   original=(await (await ctx.request.get(URL+'/api/app/me')).json())['settings']
   async def api(market,path,query):
    return await ctx.request.get(URL+'/api/'+market+'/guru/connors/'+path,params=query)
   for market,ticker in [('in','NIFTY 50'),('us','SPX')]:
    for sid in OPTIONS:
     query={'id':sid,'ticker':ticker,'from':'2020-01-01','to':'2020-06-30'}
     r=await api(market,'readiness',query);assert r.status==200
     state=(await r.json())['strategies'][0];assert state['status']!='ready',state
     states.append({'market':market,'id':sid,'status':state['status'],'note':state['note']})
     for endpoint in ['run','optimize']:
      r=await api(market,endpoint,query)
      assert r.status==400,(market,sid,endpoint,r.status)
      text=await r.text()
      assert not any('"'+key+'"' in text for key in ['equity','stats','trades','rows'])
    summary=await api(market,'summary',{'ticker':ticker,'from':'2020-01-01','to':'2020-06-30'})
    assert summary.status==200
    rows={r['id']:r for r in await summary.json()}
    for sid in OPTIONS:
     assert rows[sid].get('blocked'),sid
     assert not any(k in rows[sid] for k in ['stats','spark']),sid
    checks.append(market+': eight option variants blocked in run, optimize and summary')
   # A file existing in history is insufficient if it does not cover these requested days.
   r=await api('in','readiness',{'id':'trin4','ticker':'NIFTY 50','from':'1996-01-01','to':'1997-01-01'})
   assert r.status==200 and (await r.json())['strategies'][0]['status']=='needs_data'
   checks.append('missing breadth coverage in the selected period')
   r=await api('in','run',{'id':'otvt_cvr1','ticker':'NIFTY 50','from':'2020-01-01','to':'2021-01-01','p.src':'1'})
   assert r.status==400 and 'VXO' in await r.text()
   checks.append('unavailable volatility source cannot return an empty fake backtest')
   # Fully covered underlying-price backtests remain compatible with captured responses.
   manifest=json.loads((APP/'verification/reference/manifest.json').read_text())
   for record in manifest:
    if '-run-' not in record['file']: continue
    response=await ctx.request.get(URL+'/api/'+record['market']+'/'+record['path'].removeprefix('/api/'))
    assert response.status==200,await response.text()
    expected=json.loads((APP/'verification/reference'/record['file']).read_text());assert project_legacy(await response.json(),expected)==expected,record['file']
   checks.append('six captured India/US backtest responses match exactly')
   # Pages suppress every performance artifact for blocked results, even after a valid page.
   await page.goto(URL+'/full-variant1.html#/s/rsi2_5d?from=2020-01-01&to=2025-12-31')
   await expect(page.locator('#c-eq')).to_be_visible(timeout=30000)
   await expect(page.locator('[data-book-reference]')).to_contain_text('reference only')
   await expect(page.locator('[data-results-provenance]')).to_contain_text('Computed backtest')
   await expect(page.locator('#open-optimization')).to_be_visible(timeout=30000)
   for region,ticker,sid in [('in','NIFTY 50','oiss_short_put'),('us','SPX','oiss_short_put'),('in','NIFTY 50','otvt_cvr1_sell_puts'),('us','SPX','otvt_cvr1_buy_puts')]:
    from urllib.parse import urlencode
    await page.goto(URL+'/full-variant1.html#/s/'+sid+'?'+urlencode({'region':region,'tk':ticker,'from':'2020-01-01','to':'2020-06-30'}))
    await expect(page.locator('[data-backtest-state]')).to_be_visible(timeout=30000)
    await expect(page.locator('[data-backtest-state] .tags2')).to_contain_text('Nifty 50' if region=='in' else 'S&P 500',timeout=30000)
    expected=next(s for s in states if s.get('market')==region and s['id']==sid)
    await expect(page.locator('.gapbox')).to_contain_text(expected['note'],timeout=30000)
    await expect(page.locator('#flipov')).to_have_count(0,timeout=30000)
    await page.evaluate("Promise.all(document.getAnimations().filter(a=>a.effect.getTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{})))")
    assert await page.evaluate('S.region')==region
    assert await page.evaluate('R.id')==sid
    await expect(page.locator('.gapbox')).to_contain_text('No performance figures or charts')
    for selector in ['.metrics','#c-eq','#c-px','#c-dd','#csv','#optim','#claude-c','#yt-c','[data-book-reference]']:
     assert await page.locator(selector).count()==0,(region,sid,selector)
    states.append({'page_region':region,'id':sid,'state':await page.locator('[data-backtest-state]').get_attribute('data-backtest-state')})
    if region=='in' and sid=='oiss_short_put':
     await page.screenshot(path=str(OUT/'options-withheld-india.png'),full_page=True)
    if region=='us' and sid=='oiss_short_put':
     await page.set_viewport_size({'width':390,'height':844})
     await page.screenshot(path=str(OUT/'options-withheld-us-mobile.png'),full_page=True)
     assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth')
     await page.set_viewport_size({'width':1440,'height':1000})
   checks.append('four unavailable option pages show no charts, statistics, exports or parameter conclusions; mobile checked')
   await page.goto(URL+'/full-variant1.html#/strategies?region=us&tab=awaiting&mode=variants&q=Option%20Income')
   await expect(page.locator('#results')).to_contain_text('BACKTEST UNAVAILABLE',timeout=30000)
   assert await page.locator('#results svg').count()==0
   assert await page.locator('#results .kpis').count()==0
   checks.append('unavailable strategy list shows reasons and rule links without performance charts')
   await page.goto(URL+'/full-variant1.html#/s/rsi2_5d?from=2020-01-01&to=2025-12-31&p=hold:6')
   await expect(page.locator('#c-eq')).to_be_visible(timeout=30000)
   await expect(page.locator('.pbanner')).to_contain_text('Custom settings')
   await expect(page.locator('#open-optimization')).to_be_visible(timeout=30000)
   checks.append('custom parameters pass through the same admission checks')
   assert not errors,errors
   result={'checks':checks,'states':states,'page_errors':errors}
   (OUT/'data-readiness-report.json').write_text(json.dumps(result,indent=2))
   print(json.dumps({'checks':checks,'page_errors':errors,'recorded_states':len(states)},indent=2))
  finally:
   # Autosaving the route is normal app behavior; a verification run must restore it.
   if original is not None:
    await page.goto(URL+'/admin.html')
    await ctx.request.put(URL+'/api/app/settings',data=json.dumps(original),headers={'Origin':URL,'Content-Type':'application/json'})
   await ctx.request.post(URL+'/api/auth/sign-out',data={},headers={'Origin':URL})
   await browser.close()
asyncio.run(main())
