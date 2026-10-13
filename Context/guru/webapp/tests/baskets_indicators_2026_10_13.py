import asyncio,json
from pathlib import Path
from playwright.async_api import async_playwright,expect
CFG=dict(line.split('=',1) for line in (Path.home()/'.cache/strategy-vault-webapp/.env').read_text().splitlines() if line and not line.startswith('#'))
BASE=CFG['VAULT_PUBLIC_URL'];OUT=Path.home()/'.cache/strategy-vault-webapp/overnight-2026-10-13/browser';OUT.mkdir(parents=True,exist_ok=True)
METHODS=['kftsm_ma','kftsm_exp','kftsm_lrs','kftsm_ndb','kftsm_bb_ma','kftsm_ma_pct_band','kftsm_ma_tr_band','kftsm_triple_smooth','kftsm_vol_breakout','kftsm_keltner','kftsm_hh_ma','kftsm_hh_cross','kftsm_hh_channel','kftsm_mod_3ma','kftsm_4_9_18','kftsm_month_end','kftsm_hirsch']
async def main():
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage']);ctx=await browser.new_context(viewport={'width':1440,'height':1000},accept_downloads=True);page=await ctx.new_page();errors=[];logs=[];checks=[]
  page.on('pageerror',lambda error:errors.append(str(error)));page.on('console',lambda msg:logs.append(msg.text) if msg.type=='error' else None)
  assert (await ctx.request.get(BASE+'/api/in/ohlc?ticker=NIFTY%2050')).status==401
  assert (await ctx.request.post(BASE+'/api/auth/sign-in/email',data={'email':CFG['VAULT_ADMIN_EMAIL'],'password':CFG['VAULT_ADMIN_PASSWORD']},headers={'Origin':BASE})).status==200
  original=(await (await ctx.request.get(BASE+'/api/app/me')).json())['settings']
  try:
   await page.goto(BASE+'/#/s/r3');await expect(page.locator('.metrics')).to_be_visible(timeout=60000)
   await expect(page.locator('[data-basket="nse500"]')).to_be_disabled();await expect(page.locator('[data-instrument-note]')).to_contain_text('uses an index')
   checks.append('R3 index restriction is visible; no enabled basket button silently resets selection')
   await page.goto(BASE+'/#/s/r3?basket=nse500&tk=basket%3Anse500%3A360ONE');await expect(page.locator('[data-backtest-state]')).to_be_visible(timeout=30000)
   assert await page.locator('.metrics').count()==0;assert 'basket=nse500' in page.url
   checks.append('Explicit incompatible basket link keeps its context and shows unavailable results')
   await page.goto(BASE+'/#/s/f1a?basket=nifty50&tk=basket%3Anifty50%3ARELIANCE');await expect(page.locator('.metrics')).to_be_visible(timeout=60000)
   await page.locator('[data-basket="nse500"]').click();await expect(page.locator('[data-basket="nse500"]')).to_have_class('join-item active',timeout=60000)
   await expect(page.locator('#basket-symbol')).to_be_visible();assert 'basket=nse500' in page.url
   await expect(page.locator('#flipov')).to_have_count(0,timeout=60000)
   selected=await page.locator('#basket-symbol').input_value();assert selected.startswith('basket:nse500:')
   values=await page.locator('#basket-symbol option').evaluate_all('(els)=>els.map(e=>e.value)');await page.locator('#basket-symbol').select_option(values[1]);await expect(page.locator('#basket-symbol')).to_have_value(values[1],timeout=60000)
   await expect(page.locator('#flipov')).to_have_count(0,timeout=60000)
   checks.append('Stock method switches to NSE500 and another actual constituent without a stuck loader')
   await page.goto(BASE+'/#/s/f1a?region=us&basket=us_sp500&tk=basket%3Aus_sp500%3AAAPL');await expect(page.locator('.metrics')).to_be_visible(timeout=60000);await expect(page.locator('[data-results-provenance]')).to_contain_text('AAPL')
   await page.locator('#basket-universe').select_option('current_plus_past');await expect(page.locator('#basket-symbol')).to_have_value('basket:us_sp500_past:AAPL',timeout=60000)
   await expect(page.locator('#flipov')).to_have_count(0,timeout=60000);checks.append('US current/past basket preserves the selected actual stock')
   await page.goto(BASE+'/#/s/hpetf_r3_long?region=us&tk=QQQ');await expect(page.locator('.metrics')).to_be_visible(timeout=60000);await expect(page.locator('[data-results-provenance]')).to_contain_text('QQQ')
   checks.append('ETF method retains selected QQQ rather than silently replacing it with SPY')
   await page.goto(BASE+'/#/s/dmi_1?from=2020-01-01&to=2025-12-31');await expect(page.locator('[data-indicator-status="ready"]')).to_be_visible(timeout=60000)
   await expect(page.locator('[data-indicator-pane="plus-di"] canvas').first).to_be_visible();await expect(page.locator('[data-indicator-pane="rsi"] canvas').first).to_be_visible()
   await page.locator('[data-indicator-toggle="plus-di"]').uncheck();await expect(page.locator('[data-indicator-pane-wrap="plus-di"]')).to_be_hidden()
   await page.locator('[data-tf="W"]').click();await expect(page.locator('[data-indicator-pane-wrap="plus-di"]')).to_be_hidden();await expect(page.locator('[data-indicator-toggle="plus-di"]')).not_to_be_checked()
   await page.locator('[data-indicator-toggle="plus-di"]').check();await expect(page.locator('[data-indicator-pane-wrap="plus-di"]')).to_be_visible();await page.locator('[data-tf="D"]').click()
   await page.locator('[data-strategy-indicators]').screenshot(path=str(OUT/'dmi-indicators.png'));checks.append('DMI SMA/RSI/DI panes render and toggles survive timeframe changes')
   await page.goto(BASE+'/#/s/r3?from=2020-01-01&to=2025-12-31');await expect(page.locator('[data-indicator-status="ready"]')).to_be_visible(timeout=60000);await expect(page.locator('[data-indicator-pane="rsi"] canvas').first).to_be_visible();checks.append('R3 indicators render alongside the real trade chart')
   catalog=await (await ctx.request.get(BASE+'/api/in/guru/connors/strategies')).json();assert all(next(v for v in catalog if v['id']==m)['status']=='live' for m in METHODS)
   for method in METHODS:
    response=await ctx.request.get(BASE+'/api/in/guru/connors/run',params={'id':method,'ticker':'NIFTY 50','from':'2010-01-01','to':'2020-09-11'});assert response.status==200,(method,response.status)
    data=await response.json();closed=[t for t in data['trades'] if not t['open']];total=sum(t['ret'] for t in closed)
    # Existing run JSON rounds each return/stat to six decimals; account for its bounded serialization error.
    assert abs(data['stats']['net_profit']-total)<(len(closed)+1)*0.5e-6+1e-12,(method,'closed return sum')
    assert data['stats']['trades']==len(closed),(method,'closed count')
   await page.goto(BASE+'/#/s/kftsm_ma?from=2020-01-01&to=2025-12-31');await expect(page.locator('.metrics')).to_be_visible(timeout=60000);await expect(page.locator('[data-rule-review]')).to_be_visible()
   checks.append('17 reviewed Kaufman methods run through the authenticated API with closed-return/count reconciliation')
   await page.goto(BASE+'/#/s/kftsm_hirsch?basket=nifty50&tk=basket%3Anifty50%3ARELIANCE');await expect(page.locator('[data-backtest-state]')).to_be_visible(timeout=30000);assert await page.locator('.metrics').count()==0
   checks.append('Index/ETF seasonal rule stays blocked on an individual stock')
   await page.set_viewport_size({'width':390,'height':844});await page.goto(BASE+'/#/s/dmi_1?from=2020-01-01&to=2025-12-31');await expect(page.locator('[data-indicator-status="ready"]')).to_be_visible(timeout=60000);assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth+1');checks.append('Indicator layout fits a390pxmobile viewport')
   assert not errors,errors;assert not logs,logs
   report={'checks':checks,'page_errors':errors,'console_errors':logs};(OUT/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps({'passed':len(checks),'page_errors':errors,'console_errors':logs}))
  finally:
   await page.close();await ctx.request.put(BASE+'/api/app/settings',data=original,headers={'Origin':BASE});await ctx.request.post(BASE+'/api/auth/sign-out',data={},headers={'Origin':BASE});await browser.close()
asyncio.run(main())
