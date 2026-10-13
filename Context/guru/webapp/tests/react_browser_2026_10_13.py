import asyncio,json,os
from pathlib import Path
from playwright.async_api import async_playwright,expect
CFG=dict(line.split('=',1) for line in (Path.home()/'.cache/strategy-vault-webapp/.env').read_text().splitlines() if line and not line.startswith('#'))
BASE=os.environ.get('VAULT_REACT_TEST_URL',CFG['VAULT_PUBLIC_URL'])
AUTH=CFG['VAULT_PUBLIC_URL'];OUT=Path.home()/'.cache/strategy-vault-webapp/react-migration-2026-10-13';OUT.mkdir(exist_ok=True)
async def main():
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  context=await browser.new_context(viewport={'width':1440,'height':1000},accept_downloads=True);page=await context.new_page();errors=[];logs=[];checks=[]
  page.on('pageerror',lambda error:errors.append(str(error)));page.on('console',lambda msg:logs.append(msg.text) if msg.type=='error' else None)
  response=await context.request.post(AUTH+'/api/auth/sign-in/email',data={'email':CFG['VAULT_ADMIN_EMAIL'],'password':CFG['VAULT_ADMIN_PASSWORD']},headers={'Origin':AUTH})
  assert response.status==200,'Own verification sign-in failed'
  original=(await (await context.request.get(AUTH+'/api/app/me')).json())['settings']
  cookies=await context.cookies(AUTH)
  if BASE!=AUTH:
   for cookie in cookies:cookie['domain']='127.0.0.1'
   await context.add_cookies(cookies)
  try:
   await page.goto(BASE+'/#/');await expect(page.locator('[data-library-author]')).to_have_count(24,timeout=90000)
   await expect(page.locator('[data-react-page]')).to_have_attribute('data-react-page','home')
   await expect(page.locator('.lstrip > div').first.locator('dt')).to_have_text('Trading source groups')
   await page.locator('.library-coverage-note > summary').click();await expect(page.locator('.coverage-totals > div').filter(has_text='Curated trading collections').locator('dd')).to_have_text('71')
   await page.locator('.library-coverage-note > summary').click()
   scope=await page.get_by_label('Library scope',exact=True).bounding_box();sorting=await page.locator('#library-sort').bounding_box();assert abs(scope['y']-sorting['y'])<8
   await expect(page.get_by_label('Library scope',exact=True)).to_have_value('trading')
   await page.get_by_role('button',name='Books & reading',exact=True).click();await expect(page.locator('[data-library-book]')).to_have_count(24,timeout=30000)
   await page.locator('#library-query').fill('Engineering Matlab Programming');await expect(page.locator('.library-empty')).to_be_visible(timeout=30000)
   await page.get_by_label('Library scope',exact=True).select_option('technical');await expect(page.locator('[data-library-book]').first).to_be_visible(timeout=30000)
   await expect(page.locator('[data-library-book]').first).to_contain_text('Supporting technical reference')
   await page.locator('[data-library-book] h3 a').first.click();await expect(page.locator('[data-reading-scope]')).to_contain_text('not counted as a trading-strategy',timeout=30000)
   assert await page.locator('.metrics,#c-eq').count()==0
   checks.append('71curated collections are distinct from metadata groups; general Matlab guide is absent from trading view and readable only as a technical reference')
   await page.goto(BASE+'/#/');await expect(page.locator('[data-library-author]')).to_have_count(24,timeout=30000)

   await page.screenshot(path=str(OUT/'react-home.png'));checks.append('Native React home renders the source library and24author cards')
   await page.locator('#library-query').fill('Van Tharp');await expect(page.locator('[data-library-author="tharp"]')).to_be_visible(timeout=30000)
   await expect(page.locator('[data-library-entry-count="tharp"]')).to_have_text('19');checks.append('Search preserves reviewed Van Tharp19entry coverage')
   await page.locator('[data-library-author="tharp"] .lc-more').click();await expect(page.locator('.gmodal')).to_be_visible();await page.locator('.gm-x').click();await expect(page.locator('.gmodal')).to_have_count(0)
   await page.locator('[data-library-author="tharp"] .lc-open').click();await expect(page.locator('[data-book="tharp_super_trader"]')).to_be_visible(timeout=30000)
   assert await page.locator('[data-reading-source]').count()==16;checks.append('Author routing, original modal and reviewed book/source coverage preserved')
   await page.goto(BASE+'/#/source/source-5f239ba6b4c31bae');await expect(page.locator('[data-reading-concept-map]')).to_be_visible(timeout=30000)
   assert await page.locator('.metrics,#c-eq').count()==0;checks.append('Psychology source review has educational topics and no invented performance')
   await page.goto(BASE+'/#/s/r3?from=2020-01-01&to=2025-12-31');await expect(page.locator('.metrics')).to_be_visible(timeout=60000)
   await expect(page.locator('#c-eq canvas').first).to_be_visible();await expect(page.locator('#heatbox table')).to_be_visible();await expect(page.locator('#trades-xlsx')).to_be_visible()
   labels=await page.locator('.metric > span').all_text_contents();assert 'Avg net pnl' in labels and 'No. of trades' in labels
   async with page.expect_download(timeout=30000) as event:await page.locator('#trades-xlsx').click()
   download=await event.value;await download.save_as(str(OUT/'react-trades.xlsx'));assert (OUT/'react-trades.xlsx').stat().st_size>1000
   async with page.expect_download(timeout=30000) as event:await page.locator('[data-chart-snapshot="c-eq"]').click()
   await (await event.value).save_as(str(OUT/'react-equity.png'));assert (OUT/'react-equity.png').read_bytes().startswith(b'\x89PNG')
   await page.screenshot(path=str(OUT/'react-strategy.png'),full_page=True);checks.append('Real index run, charts, heatmap and actual XLSX/PNG exports render in React')
   await page.locator('[data-action="save"]').click();await page.get_by_role('link',name='Saved',exact=False).first.click();await expect(page.locator('.saved-row')).to_have_count(1);await page.locator('[data-unsave]').click();await expect(page.locator('.saved-row')).to_have_count(0);checks.append('Native React Saved page adds and removes browser-local saved settings')
   await page.goto(BASE+'/#/s/f1a?tk=NIFTY%2050');await expect(page.locator('main')).to_contain_text('A cash index or ETF cannot stand in',timeout=60000);assert await page.locator('.metrics,#c-eq').count()==0;checks.append('Stock-only strategy remains blocked on cash-index prices')
   await page.goto(BASE+'/#/s/f1a?basket=nifty50&tk=basket%3Anifty50%3ARELIANCE&from=2020-01-01&to=2025-12-31');await expect(page.locator('.metrics')).to_be_visible(timeout=60000);await expect(page.locator('[data-results-provenance]')).to_contain_text('RELIANCE')
   await page.set_viewport_size({'width':390,'height':844});await page.goto(BASE+'/#/');await expect(page.locator('[data-library-author]')).to_have_count(24,timeout=60000)
   assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth+1');await page.screenshot(path=str(OUT/'react-mobile.png'));checks.append('Actual stock routing and390pxmobile layout preserve behaviour')
   assert not errors,errors
   (OUT/('production-report.json' if BASE==AUTH else 'preview-report.json')).write_text(json.dumps({'base':BASE,'checks':checks,'page_errors':errors,'console_errors':logs},indent=2))
   print(json.dumps({'passed':len(checks),'page_errors':errors,'console_errors':logs[:8],'output':str(OUT)}))
  finally:
   await page.close()
   await context.request.put(AUTH+'/api/app/settings',data=original,headers={'Origin':AUTH})
   await context.request.post(AUTH+'/api/auth/sign-out',data={},headers={'Origin':AUTH})
   await browser.close()
asyncio.run(main())
