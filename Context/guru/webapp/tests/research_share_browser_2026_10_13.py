"""Bounded actual-LAN checks: source coverage, truthful reading pages, costs and book attribution."""
import asyncio,json,hashlib
from pathlib import Path
from playwright.async_api import async_playwright,expect
from openpyxl import load_workbook
from accuracy_browser_2026_10_13 import CFG,URL,ready
OUT=Path.home()/'.cache/strategy-vault-webapp/research-share-browser-2026-10-13';OUT.mkdir(exist_ok=True)

async def main():
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  context=await browser.new_context(viewport={'width':1440,'height':1050},accept_downloads=True)
  page=await context.new_page();errors=[];checks=[];original=None;counts={}
  page.on('pageerror',lambda e:errors.append(str(e)))
  try:
   await page.goto(URL+'/login.html');await page.locator('#email').fill(CFG['VAULT_ADMIN_EMAIL']);await page.locator('#password').fill(CFG['VAULT_ADMIN_PASSWORD']);await page.get_by_role('button',name='Sign in',exact=True).click();await page.wait_for_url('**/full-variant1.html**')
   original=(await (await context.request.get(URL+'/api/app/me')).json())['settings']
   await page.goto(URL+'/full-variant1.html#/');await expect(page.locator('[data-library-author]')).to_have_count(24,timeout=90000)
   await ready(page,"HOME_STATS?.connors?.tested>0 && SOURCE_REVIEWS.sources['source-c181428e93432e57'] && !document.querySelector('#flipov')")
   counts=await page.evaluate("({authors:LIB.length,entries:libraryRecords().length,tharp:libraryEntryCounts('tharp'),exactCounts:Array.from(document.querySelectorAll('[data-count]')).every(e=>e.textContent===e.dataset.count)})")
   assert counts['authors']>4000 and counts['entries']>30000 and counts['tharp']['total']>=19 and counts['exactCounts']
   await page.screenshot(path=str(OUT/'research-library.png'));checks.append('Expanded source library is indexed; exact counts and24card pages avoid full-corpus DOM rendering')
   async with page.expect_download() as dl:await page.get_by_role('link',name='Download book inventory',exact=True).click()
   excel=OUT/'downloaded-book-inventory.xlsx';await (await dl.value).save_as(str(excel));wb=load_workbook(excel,read_only=True,data_only=True)
   assert 'Books' in wb.sheetnames and any('uthor' in s for s in wb.sheetnames)
   assert wb['Books'].max_row>=11812;wb.close();counts['inventory_download_sha256']=hashlib.sha256(excel.read_bytes()).hexdigest();checks.append('Book inventory Excel downloads through the actual homepage button and opens as a real workbook')
   await page.locator('#library-query').fill('Van Tharp');await expect(page.locator('[data-library-author="tharp"]')).to_be_visible(timeout=30000)
   await expect(page.locator('[data-library-entry-count="tharp"]')).to_have_text(str(counts['tharp']['total']))
   await page.locator('[data-library-author="tharp"] .lc-open').click();await expect(page.locator('[data-book="tharp_super_trader"]')).to_be_visible(timeout=30000)
   await expect(page.locator('[data-book="tharp_system_workbook"]')).to_be_visible();await expect(page.locator('[data-book-coverage] > details')).to_have_attribute('open','')
   assert await page.locator('[data-reading-source]').count()==counts['tharp']['reading']
   await page.screenshot(path=str(OUT/'tharp-library.png'),full_page=True);checks.append('Van Tharp card counts reading material too; three source-specific catalogue entries and all remaining sources are visible')
   await page.goto(URL+'/full-variant1.html#/source/source-d310cdc2f7752e88');await expect(page.locator('[data-reading-review]')).to_be_visible(timeout=30000)
   await expect(page.locator('.reading-credit')).to_contain_text('Brian June')
   assert await page.locator('.metrics,#c-eq').count()==0
   link=page.locator('.reading-page .source-pdfs a').first;res=await context.request.get(URL+await link.get_attribute('href'),headers={'Range':'bytes=0-63'});assert res.status==206 and (await res.body()).startswith(b'%PDF-')
   await page.goto(URL+'/full-variant1.html#/source/source-5f239ba6b4c31bae');await expect(page.locator('[data-reading-concept-map]')).to_be_visible(timeout=30000)
   await expect(page.locator('.book-types')).to_contain_text('Psychology');assert await page.locator('.metrics,#c-eq').count()==0
   await page.screenshot(path=str(OUT/'psychology-reading.png'),full_page=True);checks.append('Source pages preserve actual coauthors, authenticated PDF bytes and psychology summaries/topic diagrams without performance charts')
   await page.goto(URL+'/full-variant1.html#/?library_view=books');await expect(page.locator('[data-library-book]')).to_have_count(24,timeout=90000);first=await page.locator('[data-library-book]').evaluate_all('(els)=>els.map(e=>e.dataset.libraryBook)')
   await page.get_by_role('button',name='Next 24 books →',exact=True).click();await ready(page,"S.library_page==='2'");await expect(page.locator('[data-library-book]')).to_have_count(24)
   second=await page.locator('[data-library-book]').evaluate_all('(els)=>els.map(e=>e.dataset.libraryBook)');assert not set(first)&set(second)
   pending=await page.evaluate("libraryRecords().find(r=>r.kind==='source' && r.source.local_status==='original_copy_pending').id")
   await page.goto(URL+'/full-variant1.html#/source/'+pending);await expect(page.locator('[data-reading-page]')).to_be_visible(timeout=30000)
   await expect(page.locator('.reading-page')).to_contain_text('verified local reading copy is not available');assert await page.locator('.reading-page .source-pdfs a,.metrics,#c-eq').count()==0
   checks.append('Paged reading entries are distinct; uncached source copies are labelled missing and have no fabricated reader link or results')
   response=await context.request.get(URL+'/api/in/guru/connors/strategies');defs={d['id']:d for d in await response.json()}
   assert defs['tharp_110day_breakout']['book']=='tharp_super_trader' and defs['tharp_false_breakout']['book']=='tharp_system_workbook'
   assert 'alpha = 2/(n+1)' in ' '.join(defs['tharp_random_entry']['rules'])
   blocked=await context.request.get(URL+'/api/in/guru/connors/run?id=tharp_random_entry&ticker=NIFTY%2050&from=2000-01-03&to=2026-10-07');assert blocked.status==400 and 'equity' not in (await blocked.text()).lower()
   actual=await context.request.get(URL+'/api/in/guru/connors/run?id=f1a&ticker=basket%3Anifty50%3ARELIANCE&from=2020-01-01&to=2025-12-31&cost=0&slippage=3');assert actual.status==200
   result=await actual.json();s=result['stats'];assert s['trades']>0 and abs(s['net_profit']-s['avg_net_pnl']*s['trades'])<.0002
   checks.append('Live backend assigns methods to actual source books, exposes repaired EMA specification, withholds unresolved Tharp performance and preserves actual-stock arithmetic')
   await page.set_viewport_size({'width':390,'height':844});await page.goto(URL+'/full-variant1.html#/?library_q=Van%20Tharp');await expect(page.locator('[data-library-author="tharp"]')).to_be_visible(timeout=90000)
   assert await page.evaluate('document.documentElement.scrollWidth<=window.innerWidth+1')
   await page.screenshot(path=str(OUT/'mobile-tharp.png'),full_page=True);assert not errors,errors;checks.append('Mobile source search/actions fit390px without browser errors')
  except Exception:
   await page.screenshot(path=str(OUT/'failure.png'),full_page=True);raise
  finally:
   try:
    if original is not None:await context.request.put(URL+'/api/app/settings',data=json.dumps(original),headers={'Origin':URL,'Content-Type':'application/json'})
    response=await context.request.post(URL+'/api/auth/sign-out',data='{}',headers={'Origin':URL,'Content-Type':'application/json'});assert response.status==200
    report={'checks':checks,'counts':counts,'page_errors':errors,'test_session_signed_out':True};(OUT/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
   finally:await context.close();await browser.close()

asyncio.run(main())
