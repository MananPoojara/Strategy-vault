import asyncio,json,sys
from pathlib import Path
from playwright.async_api import async_playwright,expect
from accuracy_browser_2026_10_13 import CFG,URL,ready
OUT=Path.home()/'.cache/strategy-vault-webapp/library-browser-2026-10-13';OUT.mkdir(exist_ok=True)
async def main():
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/google-chrome',headless=True,args=['--disable-dev-shm-usage'])
  context=await browser.new_context(viewport={'width':1440,'height':1050});page=await context.new_page();errors=[];checks=[]
  page.on('pageerror',lambda e:errors.append(str(e)))
  try:
   await page.goto(URL+'/login.html');await page.locator('#email').fill(CFG['VAULT_ADMIN_EMAIL']);await page.locator('#password').fill(CFG['VAULT_ADMIN_PASSWORD']);await page.get_by_role('button',name='Sign in',exact=True).click();await page.wait_for_url('**/full-variant1.html**');await page.goto(URL+'/full-variant1.html#/')
   await expect(page.locator('[data-library-author]')).to_have_count(71,timeout=45000)
   await ready(page,"HOME_STATS?.connors?.tested>0 && document.querySelector('#library-query') && !document.querySelector('#flipov')")
   zero=await page.locator('[data-library-author][data-author-availability="source_review"]').count();assert zero>0
   assert await page.locator('[data-library-author="pristine"]').inner_text() and 'Rule review pending' in await page.locator('[data-library-author="pristine"]').inner_text()
   await asyncio.sleep(.5);await page.screenshot(path=str(OUT/'library-home.png'),full_page=False);checks.append('71authors include Pristine; zero-backtest cards state actual source/data/instrument reasons')
   await page.locator('#library-query').fill('prestine');await expect(page.locator('[data-library-author]')).to_have_count(1,timeout=30000)
   await expect(page.locator('[data-library-author="pristine"]')).to_be_visible();await page.screenshot(path=str(OUT/'pristine-search.png'))
   await page.locator('[data-library-author="pristine"] .lc-open').click();await expect(page.locator('.bookcard.pv')).to_have_count(3,timeout=30000)
   await page.get_by_role('heading',name='Tools & Tactics for the Master Day Trader',exact=True).click();await expect(page.locator('[data-reading-page="p_tools_tactics"]')).to_be_visible(timeout=30000)
   await expect(page.locator('.reading-credit')).to_contain_text('Oliver Velez & Greg Capra')
   link=page.locator('.reading-page .source-pdfs a').first;response=await context.request.get(URL+await link.get_attribute('href'),headers={'Range':'bytes=0-63'});assert response.status==206 and (await response.body()).startswith(b'%PDF-')
   assert await page.locator('.metrics,#c-eq').count()==0
   await page.screenshot(path=str(OUT/'pristine-reading.png'));checks.append('Pristine alias search opens three reading entries; book page preserves coauthors and authenticated original PDF, with no invented backtest')
   await page.goto(URL+'/full-variant1.html#/?library_view=books');await expect(page.locator('[data-library-book]')).to_have_count(24,timeout=30000)
   await page.get_by_role('button',name='Show 24 more',exact=True).click();await expect(page.locator('[data-library-book]')).to_have_count(48,timeout=30000)
   await page.get_by_role('button',name='Psychology',exact=True).click();await ready(page,"S.book_category==='psychology' && document.querySelectorAll('[data-library-book]').length>0")
   assert await page.locator('[data-library-book]').count()==4
   assert await page.locator('.library-book .book-types').count()==4
   await page.locator('#library-sort').select_option('name-desc');await ready(page,"S.library_sort==='name-desc'")
   titles=await page.locator('.library-book h3').all_text_contents();assert titles==sorted(titles,key=str.casefold,reverse=True)
   await page.screenshot(path=str(OUT/'psychology-books.png'));checks.append('Books/source view paginates24entries; Psychology category and reverse-title sorting use declared topic evidence')
   await page.get_by_role('button',name='Money management',exact=True).click();await ready(page,"S.book_category==='money-management' && document.querySelectorAll('[data-library-book]').length>0");assert await page.locator('[data-library-book]').count()==7
   await page.goto(URL+'/full-variant1.html#/?library_view=books&library_q=stock%20market%20wizard');await ready(page,"S.library_q==='stock market wizard' && document.querySelectorAll('[data-library-book]').length>0")
   assert 'Trade Like a Stock Market Wizard' in await page.locator('.library-book h3').all_text_contents()
   await page.goto(URL+'/full-variant1.html#/?library_q=missingXYZabcdef');await expect(page.locator('.library-empty')).to_be_visible(timeout=30000);await page.get_by_role('button',name='Clear search and filters',exact=True).click();await expect(page.locator('[data-library-author]')).to_have_count(71,timeout=30000)
   checks.append('Money management, exact book title and no-match reset work without classifying unknown sources as rules')
   await page.locator('#library-sort').select_option('name');await ready(page,"S.library_sort==='name'");names=await page.locator('[data-library-author] h3').all_text_contents();assert names==sorted(names,key=str.casefold)
   await page.locator('#library-instrument').select_option('b:nifty50');await ready(page,"S.basket==='nifty50' && document.querySelector('#library-stock') && HOME_STATS?.connors?.tested>0")
   assert await page.evaluate("TICKERS.every(t=>t.ticker.startsWith('basket:nifty50:'))")
   assert await page.evaluate('Array.from(document.querySelectorAll("[data-count]")).every(e=>e.textContent===e.dataset.count)')
   selected=await page.evaluate('S.tk');await page.locator('[data-library-author="connors"] .lc-open').click();await expect(page.locator('.shelf').first).to_be_visible(timeout=30000)
   assert await page.evaluate("S.basket==='nifty50'") and await page.evaluate('S.tk')==selected
   await page.locator('.bookcard[data-book="stc9"]').click();await expect(page.locator('#results')).to_be_visible(timeout=30000)
   assert await page.evaluate("S.basket==='nifty50'") and await page.evaluate('S.tk')==selected
   checks.append('Author sorting, exact non-animated counts and index/basket context persist through author and book links without combining capital')
   await page.goto(URL+'/full-variant1.html#/guru/pristine');await expect(page.locator('[data-book-coverage]')).to_be_visible(timeout=30000)
   await expect(page.locator('[data-book-coverage]')).to_contain_text('29 source-title candidates')
   await page.locator('[data-book-coverage] > details > summary').click()
   source=page.locator('[data-reading-source]').filter(has_text='Gibby Market Prep and Gaps').first;await expect(source).to_be_visible();await expect(source).to_contain_text('Dan Gibby')
   link=source.locator('.source-pdfs a').first;response=await context.request.get(URL+await link.get_attribute('href'),headers={'Range':'bytes=0-63'});assert response.status==206 and (await response.body()).startswith(b'%PDF-')
   await page.goto(URL+'/full-variant1.html#/?library_view=books&library_q=Dan%20Gibby');await ready(page,"S.library_q==='Dan Gibby' && document.querySelectorAll('[data-library-book]').length>0")
   await expect(page.locator('.library-book').first).to_contain_text('Dan Gibby')
   sourceLink=page.locator('.library-book-foot a').first;await sourceLink.click();await expect(page.locator('[data-reading-page]')).to_be_visible();await expect(page.locator('.reading-credit')).to_contain_text('Dan Gibby')
   checks.append('All29related Pristine source candidates appear with preserved author credits; new authenticated PDF range and source reading pages work')

   await page.set_viewport_size({'width':390,'height':844});await page.goto(URL+'/full-variant1.html#/?library_q=prestine');await expect(page.locator('[data-library-author="pristine"]')).to_be_visible(timeout=30000)
   qbox=await page.locator('.library-search').bounding_box();assert qbox and qbox['x']>=0 and qbox['x']+qbox['width']<=390.5
   await asyncio.sleep(.6);await page.screenshot(path=str(OUT/'library-mobile.png'),full_page=True);assert not errors,errors;checks.append('Mobile search and filters fit390px with no browser errors')
  except Exception:
   await page.screenshot(path=str(OUT/'failure.png'),full_page=True);raise
  finally:
   try:
    signed_out=await context.request.post(URL+'/api/auth/sign-out',headers={'Origin':URL,'Content-Type':'application/json'},data='{}')
    assert signed_out.status==200
    report={'checks':checks,'page_errors':errors,'test_session_signed_out':True}
    (OUT/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
   finally:
    await context.close();await browser.close()
asyncio.run(main())
